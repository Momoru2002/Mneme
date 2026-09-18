import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type ViewMode, useEditorStore } from './editor.ts';

export type Tab = { wellId: string; path: string };

export type GroupNode = {
  type: 'group';
  id: string;
  tabs: Tab[];
  activeTab: number | null;
  viewMode: ViewMode;
};

export type SplitNode = {
  type: 'split';
  id: string;
  direction: 'row' | 'col';
  sizes: number[];
  children: LayoutNode[];
};

export type LayoutNode = GroupNode | SplitNode;

// Deterministic id generator (no Math.random — keeps tests stable & avoids the
// sandbox ban on Math.random/Date.now). Module-level counter.
let idSeq = 0;
function nextId(prefix: string): string {
  idSeq += 1;
  return `${prefix}${idSeq}`;
}

export function makeGroup(viewMode: ViewMode = 'editor'): GroupNode {
  return { type: 'group', id: nextId('g'), tabs: [], activeTab: null, viewMode };
}

export type WorkspaceState = {
  root: LayoutNode;
  activeGroupId: string;

  openInActiveGroup: (tab: Tab) => void;
  closeTab: (groupId: string, index: number) => void;
  setActiveTab: (groupId: string, index: number) => void;
  setActiveGroup: (groupId: string) => void;
  setGroupViewMode: (groupId: string, mode: ViewMode) => void;
  splitGroup: (groupId: string, direction: 'row' | 'col') => void;
  closeGroup: (groupId: string) => void;
  closeAllTabs: () => void;
  resize: (splitId: string, sizes: number[]) => void;

  removePathFromTabs: (wellId: string, path: string) => void;
  renamePathInTabs: (wellId: string, oldPath: string, newPath: string) => void;
  migrateLegacyOpenPath: (wellId: string, path: string) => void;

  selectOpenPaths: (wellId: string) => Set<string>;
  selectActivePath: () => Tab | null;
};

export function initialWorkspace(): Pick<WorkspaceState, 'root' | 'activeGroupId'> {
  // Seed the initial root group from the user's saved view-mode preference.
  // Both stores are sync-hydrated zustand, so reading editor state here is safe.
  const g = makeGroup(useEditorStore.getState().defaultViewMode);
  return { root: g, activeGroupId: g.id };
}

// --- pure tree helpers -----------------------------------------------------

export function mapTree(node: LayoutNode, fn: (g: GroupNode) => GroupNode): LayoutNode {
  if (node.type === 'group') return fn(node);
  return { ...node, children: node.children.map((c) => mapTree(c, fn)) };
}

export function findGroup(node: LayoutNode, id: string): GroupNode | null {
  if (node.type === 'group') return node.id === id ? node : null;
  for (const c of node.children) {
    const f = findGroup(c, id);
    if (f) return f;
  }
  return null;
}

export function eachGroup(node: LayoutNode, fn: (g: GroupNode) => void): void {
  if (node.type === 'group') fn(node);
  else for (const c of node.children) eachGroup(c, fn);
}

const sameTab = (a: Tab, b: Tab) => a.wellId === b.wellId && a.path === b.path;

/**
 * Defensive hydration merge for the persist middleware. A corrupt persisted
 * tree must degrade to a clean empty group instead of crashing the render (a
 * white screen). Pure except for the deliberate `idSeq` reseed, and safe on
 * undefined/null persisted state.
 */
export function mergeWorkspace(persisted: unknown, current: WorkspaceState): WorkspaceState {
  const p = persisted as Partial<WorkspaceState> | undefined;
  const valid =
    !!p?.root &&
    (p.root.type === 'group' || p.root.type === 'split') &&
    typeof p.activeGroupId === 'string';
  if (!valid) return { ...current, ...initialWorkspace() };
  // Re-seed idSeq above the max numeric suffix across all persisted node ids so
  // a freshly generated id (g13/s4/…) never collides with a restored one.
  let max = 0;
  const scan = (n: LayoutNode): void => {
    const m = /\d+$/.exec(n.id);
    if (m) max = Math.max(max, Number(m[0]));
    if (n.type === 'split') n.children.forEach(scan);
  };
  scan(p.root as LayoutNode);
  idSeq = Math.max(idSeq, max);
  return { ...current, root: p.root as LayoutNode, activeGroupId: p.activeGroupId as string };
}

export const useWorkspaceStore = create<WorkspaceState>()(
  persist(
    (set, get) => ({
      ...initialWorkspace(),

      openInActiveGroup: (tab) =>
        set((s) => {
          const activeId = s.activeGroupId;
          const root = mapTree(s.root, (g) => {
            if (g.id !== activeId) return g;
            const idx = g.tabs.findIndex((t) => sameTab(t, tab));
            if (idx >= 0) return { ...g, activeTab: idx };
            const tabs = [...g.tabs, tab];
            return { ...g, tabs, activeTab: tabs.length - 1 };
          });
          return { root };
        }),

      closeTab: (groupId, index) =>
        set((s) => {
          const root = mapTree(s.root, (g) => {
            if (g.id !== groupId) return g;
            const tabs = g.tabs.filter((_, i) => i !== index);
            let activeTab: number | null = g.activeTab;
            if (tabs.length === 0) activeTab = null;
            else if (g.activeTab === null) activeTab = 0;
            else if (index < g.activeTab) activeTab = g.activeTab - 1;
            else if (index === g.activeTab) activeTab = Math.min(g.activeTab, tabs.length - 1);
            return { ...g, tabs, activeTab };
          });
          return { root };
        }),

      setActiveTab: (groupId, index) =>
        set((s) => ({
          root: mapTree(s.root, (g) => (g.id === groupId ? { ...g, activeTab: index } : g)),
          activeGroupId: groupId,
        })),

      setActiveGroup: (groupId) => set({ activeGroupId: groupId }),

      setGroupViewMode: (groupId, mode) =>
        set((s) => ({
          root: mapTree(s.root, (g) => (g.id === groupId ? { ...g, viewMode: mode } : g)),
        })),

      splitGroup: (groupId, direction) =>
        set((s) => {
          const newGroup = makeGroup(useEditorStore.getState().defaultViewMode);
          const replace = (node: LayoutNode): LayoutNode => {
            if (node.type === 'group') {
              if (node.id !== groupId) return node;
              return {
                type: 'split',
                id: nextId('s'),
                direction,
                sizes: [50, 50],
                children: [node, newGroup],
              };
            }
            return { ...node, children: node.children.map(replace) };
          };
          return { root: replace(s.root), activeGroupId: newGroup.id };
        }),

      closeGroup: (groupId) =>
        set((s) => {
          // Remove the group; if its parent split drops to one child, collapse it.
          const prune = (node: LayoutNode): LayoutNode | null => {
            if (node.type === 'group') return node.id === groupId ? null : node;
            const kids = node.children.map(prune).filter((n): n is LayoutNode => n !== null);
            if (kids.length === 0) return null;
            if (kids.length === 1) return kids[0] ?? null; // collapse single-child split
            // Re-normalise sizes to equal shares after a removal.
            const share = Math.round((100 / kids.length) * 100) / 100;
            const sizes = kids.map(() => share);
            return { ...node, children: kids, sizes };
          };
          const prunedRoot = prune(s.root);
          const root = prunedRoot ?? makeGroup(useEditorStore.getState().defaultViewMode);
          // Pick a new active group: first group found in the pruned tree.
          let active: string | null = null;
          eachGroup(root, (g) => {
            if (active === null) active = g.id;
          });
          return { root, activeGroupId: active ?? (root as GroupNode).id };
        }),

      closeAllTabs: () =>
        set(() => {
          // Global "close all editors": drop every tab in every pane and collapse
          // any splits back to a single fresh group, honouring the saved view-mode
          // preference (mirrors the closeGroup empty-tree fallback).
          const g = makeGroup(useEditorStore.getState().defaultViewMode);
          return { root: g, activeGroupId: g.id };
        }),

      resize: (splitId, sizes) =>
        set((s) => {
          const apply = (node: LayoutNode): LayoutNode => {
            if (node.type === 'group') return node;
            if (node.id === splitId) return { ...node, sizes };
            return { ...node, children: node.children.map(apply) };
          };
          return { root: apply(s.root) };
        }),

      removePathFromTabs: (wellId, path) =>
        set((s) => ({
          root: mapTree(s.root, (g) => {
            const removedIdx = g.tabs.findIndex((t) => t.wellId === wellId && t.path === path);
            if (removedIdx === -1) return g;
            const keep = g.tabs.filter((_, i) => i !== removedIdx);
            let activeTab: number | null = g.activeTab;
            if (keep.length === 0) activeTab = null;
            else if (g.activeTab === null) activeTab = 0;
            else if (removedIdx < g.activeTab) activeTab = g.activeTab - 1;
            else activeTab = Math.min(g.activeTab, keep.length - 1);
            return { ...g, tabs: keep, activeTab };
          }),
        })),

      renamePathInTabs: (wellId, oldPath, newPath) =>
        set((s) => ({
          root: mapTree(s.root, (g) => ({
            ...g,
            tabs: g.tabs.map((t) =>
              t.wellId === wellId && t.path === oldPath ? { ...t, path: newPath } : t,
            ),
          })),
        })),

      migrateLegacyOpenPath: (wellId, path) =>
        set((s) => {
          // Only seed when the entire workspace is a single empty root group.
          if (s.root.type !== 'group' || s.root.tabs.length > 0) return s;
          const tabs = [{ wellId, path }];
          // Honour the user's saved view-mode preference for this fresh seed.
          const viewMode = useEditorStore.getState().defaultViewMode;
          return { root: { ...s.root, tabs, activeTab: 0, viewMode } };
        }),

      selectOpenPaths: (wellId) => {
        const out = new Set<string>();
        eachGroup(get().root, (g) => {
          for (const t of g.tabs) if (t.wellId === wellId) out.add(t.path);
        });
        return out;
      },

      selectActivePath: () => {
        const s = get();
        const g = findGroup(s.root, s.activeGroupId);
        if (!g || g.activeTab === null) return null;
        return g.tabs[g.activeTab] ?? null;
      },
    }),
    {
      name: 'mneme.workspace',
      // Persist only the layout; selectors/actions are derived.
      partialize: (s) => ({ root: s.root, activeGroupId: s.activeGroupId }),
      // Defensive hydration: a malformed persisted tree degrades to a fresh
      // empty group, and idSeq is reseeded so new ids never collide.
      merge: (persisted, current) => mergeWorkspace(persisted, current as WorkspaceState),
    },
  ),
);
