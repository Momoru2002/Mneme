import { create } from 'zustand';
import type { SaveScheduler } from '../lib/save-scheduler.ts';
import type { SaveStatus } from '../lib/save-status.ts';

/** Stable identity for an open file across panes. */
export function docKeyOf(wellId: string, path: string): string {
  return `${wellId}::${path}`;
}

/** Reactive, per-document view state mirrored by the DocumentHost and read by panes. */
export type DocView = {
  refCount: number;
  buffer: string;
  status: SaveStatus;
  dirty: boolean;
  /** Server hash of the last primed/saved content (anti-clobber). */
  lastSavedHash: string | null;
  /** True once the file content has loaded at least once. */
  ready: boolean;
  /** Reactive flag the view observes to open/close the MergeDialog. */
  mergeOpen: boolean;
};

const EMPTY_VIEW: Omit<DocView, 'refCount'> = {
  buffer: '',
  status: { kind: 'idle' },
  dirty: false,
  lastSavedHash: null,
  ready: false,
  mergeOpen: false,
};

type DocumentsState = {
  docs: Record<string, DocView>;
  acquire: (wellId: string, path: string) => void;
  release: (wellId: string, path: string) => Promise<void>;
  patchView: (key: string, patch: Partial<Omit<DocView, 'refCount'>>) => void;
};

// --- Imperative side tables (NOT reactive: schedulers + host action handles) ---
// Kept outside zustand state so per-keystroke scheduling and CodeMirror view
// registration don't churn React subscribers.

const schedulers = new Map<string, SaveScheduler>();
export function registerScheduler(key: string, s: SaveScheduler): void {
  schedulers.set(key, s);
}
export function schedulerOf(key: string): SaveScheduler | undefined {
  return schedulers.get(key);
}
function deleteScheduler(key: string): void {
  schedulers.delete(key);
}

/**
 * Imperative actions a DocumentHost exposes so the (later) pane view can drive
 * the engine — save now, resolve a conflict, insert a template — without the
 * view owning any of the save/conflict logic.
 */
export type DocActions = {
  onChange: (next: string) => void;
  onForceSave: () => void;
  reloadFromDisk: () => void;
  overwriteDisk: () => void;
  openMerge: () => void;
  /** Resolve the on-disk conflict with the merged body (force-saves it). */
  resolveMerge: (mergedBody: string) => void;
  /** Close the MergeDialog without resolving (clears `mergeOpen`). */
  closeMerge: () => void;
  insertTemplate: (rendered: string) => void;
  /** Latest merge "disk" content, for the MergeDialog. */
  getMergeDisk: () => string;
};

const actionsMap = new Map<string, DocActions>();
export function registerActions(key: string, a: DocActions): void {
  actionsMap.set(key, a);
}
export function actionsOf(key: string): DocActions | undefined {
  return actionsMap.get(key);
}
export function unregisterActions(key: string): void {
  actionsMap.delete(key);
}

export const useDocumentsStore = create<DocumentsState>()((set, get) => ({
  docs: {},
  acquire: (wellId, path) => {
    const key = docKeyOf(wellId, path);
    set((state) => {
      const existing = state.docs[key];
      const next: DocView = existing
        ? { ...existing, refCount: existing.refCount + 1 }
        : { refCount: 1, ...EMPTY_VIEW };
      return { docs: { ...state.docs, [key]: next } };
    });
  },
  release: async (wellId, path) => {
    const key = docKeyOf(wellId, path);
    const current = get().docs[key];
    if (!current) return;
    if (current.refCount > 1) {
      set((state) => ({
        docs: { ...state.docs, [key]: { ...current, refCount: current.refCount - 1 } },
      }));
      return;
    }
    // Decrement to 0 before the async flush so that any concurrent acquire
    // increments from 0 → 1 instead of from 1 → 2.
    set((state) => {
      const doc = state.docs[key];
      if (!doc) return state;
      return { docs: { ...state.docs, [key]: { ...doc, refCount: 0 } } };
    });
    // Last reference: flush the pending save, then tear the entry + scheduler down.
    await schedulers.get(key)?.flush();
    // A concurrent acquire may have incremented refCount during the flush; if so,
    // abort teardown — the doc is live again.
    if ((get().docs[key]?.refCount ?? 0) > 0) return;
    deleteScheduler(key);
    set((state) => {
      const { [key]: _gone, ...rest } = state.docs;
      return { docs: rest };
    });
  },
  patchView: (key, patch) => {
    set((state) => {
      const existing = state.docs[key];
      // A mounted DocumentHost is the authority for its doc's view-state, so its
      // projection must never be silently lost. The host's prime effect (a child)
      // can fire BEFORE EditorWorkbench's acquire effect (its parent) — React runs
      // child effects before parent ones — so when a file's content is already
      // cached the buffer write can arrive before the entry exists. Upsert with
      // refCount 0 so the pending acquire still transitions 0→1 (no double count)
      // and the primed buffer survives. (Dropping it here left a freshly-split
      // second pane permanently blank: the prime landed pre-acquire, was discarded,
      // and file.data never changed again to re-trigger it.)
      const base = existing ?? { refCount: 0, ...EMPTY_VIEW };
      return { docs: { ...state.docs, [key]: { ...base, ...patch } } };
    });
  },
}));
