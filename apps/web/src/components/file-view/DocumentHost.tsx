import { useCallback, useEffect, useRef } from 'react';
import { ApiError, api } from '../../lib/api-client.ts';
import { reconstructContent, shouldPrimeBuffer } from '../../lib/editor-buffer.ts';
import { useFileContent, useUpdateFile } from '../../lib/files.ts';
import { createSaveScheduler } from '../../lib/save-scheduler.ts';
import type { SaveStatus } from '../../lib/save-status.ts';
import { usePrefs } from '../../lib/settings.ts';
import {
  docKeyOf,
  registerActions,
  registerScheduler,
  unregisterActions,
  useDocumentsStore,
} from '../../stores/documents.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';

type Props = { wellId: string; path: string };

/**
 * The single owner of one open file's edit engine. Renders nothing. One instance
 * is mounted per open file (NOT per pane) so there is exactly one save scheduler
 * and one buffer per path — two panes on the same file share this host, which is
 * what prevents the dual-scheduler conflict storm (see design doc).
 */
export function DocumentHost({ wellId, path }: Props): null {
  const key = docKeyOf(wellId, path);
  const patchView = useDocumentsStore((s) => s.patchView);
  const file = useFileContent(wellId, path);
  const update = useUpdateFile(wellId);
  const { autoSaveMs } = usePrefs();
  const removePathFromTabs = useWorkspaceStore((s) => s.removePathFromTabs);

  // The host's working state. The registry (patchView) is the read-only
  // projection panes consume; these refs remain the source of truth here.
  const bufferRef = useRef('');
  const lastSavedHashRef = useRef<string | null>(null);
  const dirtyRef = useRef(false);
  const mergeDiskRef = useRef('');

  const setBuffer = useCallback(
    (next: string) => {
      bufferRef.current = next;
      patchView(key, { buffer: next });
    },
    [key, patchView],
  );
  const setStatus = useCallback(
    (status: SaveStatus) => patchView(key, { status }),
    [key, patchView],
  );

  // The three-step "a write landed" commit: pin the hash we just persisted, clear
  // the dirty flag, and project both into the registry. Shared by every save path.
  const markSaved = useCallback(
    (hash: string) => {
      lastSavedHashRef.current = hash;
      dirtyRef.current = false;
      patchView(key, { dirty: false, lastSavedHash: hash });
    },
    [key, patchView],
  );

  // The actual write. Reads the freshest frontmatter prefix from the cache so a
  // forced/Cmd-S save uses the same reconstruction as autosave.
  const performSave = useCallback(
    async (fullContent: string): Promise<void> => {
      const expectedHash = lastSavedHashRef.current ?? undefined;
      setStatus({ kind: 'saving' });
      try {
        const result = await update.mutateAsync({ path, content: fullContent, expectedHash });
        markSaved(result.hash);
        setStatus({ kind: 'saved', at: Date.now() });
      } catch (err) {
        const isConflict = err instanceof ApiError && err.code === 'conflict';
        if (isConflict) {
          // Pause autosave (stop the stale-hash retry loop); keep the dirty buffer.
          setStatus({ kind: 'conflict' });
        } else {
          setStatus({ kind: 'error', message: err instanceof Error ? err.message : 'Save failed' });
        }
      }
    },
    [path, update, setStatus, markSaved],
  );

  // The scheduler is keyed ONLY on (key, autoSaveMs). performSave and file.data
  // are read through refs: performSave's dep `update` (from useMutation) is a
  // fresh object every render, and file.data is re-identified on every refetch /
  // setQueryData. Putting either in the scheduler effect's deps would rebuild —
  // and thus flush — the scheduler on every render, defeating the debounce and
  // firing a save on every keystroke.
  const schedulerRef = useRef<ReturnType<typeof createSaveScheduler> | null>(null);
  const performSaveRef = useRef(performSave);
  const fileDataRef = useRef(file.data);
  useEffect(() => {
    performSaveRef.current = performSave;
    fileDataRef.current = file.data;
  });

  // key (wellId::path) is a lifecycle key — a host is keyed by path so this only mounts/unmounts per file; the cleanup's flush therefore saves the OUTGOING file's pending edit (EDITING-3). performSave + file.data are read via the refs above so the scheduler is NOT rebuilt (and flushed) on every render. React runs all effect cleanups before all setups, so on a file switch the cleanup's synchronous reads of those refs still see the outgoing file. (Ported from EditorWorkbench's scheduler effect; the original needed a biome-ignore because openPath was an unused dep — here `key` is genuinely read via registerScheduler, so no suppression is required.)
  useEffect(() => {
    const reconstruct = (nextBody: string): string => {
      const data = fileDataRef.current;
      if (!data) return nextBody;
      return reconstructContent(data.content, data.body, nextBody);
    };
    const scheduler = createSaveScheduler(
      (body) => performSaveRef.current(reconstruct(body)),
      autoSaveMs,
    );
    schedulerRef.current = scheduler;
    registerScheduler(key, scheduler);
    return () => {
      // Flush the pending edit when switching files / unmounting (EDITING-3).
      void scheduler.flush();
      schedulerRef.current = null;
    };
  }, [key, autoSaveMs]);

  // Flush on app blur + tab hide (covers app-switch / minimize). Tauri window
  // close is best-effort via a guarded dynamic import (no-op in plain browser).
  useEffect(() => {
    const flush = () => {
      void schedulerRef.current?.flush();
    };
    window.addEventListener('blur', flush);
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') flush();
    };
    document.addEventListener('visibilitychange', onVisibility);

    let unlisten: (() => void) | undefined;
    void import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => getCurrentWindow().onCloseRequested(() => flush()))
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        /* not running inside Tauri (dev browser / tests) */
      });

    return () => {
      window.removeEventListener('blur', flush);
      document.removeEventListener('visibilitychange', onVisibility);
      unlisten?.();
    };
  }, []);

  // Prime the buffer from server data — but NEVER clobber unsaved edits (STATES-1
  // anti-clobber). The explicit `status.kind === 'conflict'` early-return from the
  // original is intentionally dropped: a conflict leaves the buffer dirty, and
  // shouldPrimeBuffer returns false whenever isDirty, so re-priming is still
  // suppressed during an unresolved conflict.
  useEffect(() => {
    if (!file.data) return;
    const prime = shouldPrimeBuffer({
      isDirty: dirtyRef.current,
      serverHash: file.data.hash,
      lastSavedHash: lastSavedHashRef.current,
    });
    patchView(key, { ready: true });
    if (!prime) return;
    lastSavedHashRef.current = file.data.hash;
    dirtyRef.current = false;
    setBuffer(file.data.body);
    patchView(key, { dirty: false, lastSavedHash: file.data.hash, status: { kind: 'idle' } });
  }, [file.data, key, patchView, setBuffer]);

  // Per-tab stale guard: when the file no longer exists on disk the read query
  // surfaces a not-found ApiError. Drop this tab so a deleted-on-disk file
  // self-heals. Only a genuine missing-file signal triggers removal — a
  // transient network/server error classifies as 'unknown' (see api-client
  // `classify`), so a server hiccup never nukes open tabs.
  useEffect(() => {
    if (!file.error) return;
    const notFound = file.error instanceof ApiError && file.error.code === 'not_found';
    if (notFound) removePathFromTabs(wellId, path);
  }, [file.error, wellId, path, removePathFromTabs]);

  const onChange = useCallback(
    (next: string) => {
      setBuffer(next);
      dirtyRef.current = true;
      patchView(key, { dirty: true });
      setStatus({ kind: 'dirty' });
      schedulerRef.current?.schedule(next);
    },
    [key, patchView, setBuffer, setStatus],
  );

  const onForceSave = useCallback(() => {
    void schedulerRef.current?.flush();
  }, []);

  // Conflict actions ---------------------------------------------------------
  const reloadFromDisk = useCallback(async () => {
    const fresh = await api.files.read(wellId, path);
    setBuffer(fresh.body);
    markSaved(fresh.hash);
    setStatus({ kind: 'idle' });
  }, [wellId, path, setBuffer, setStatus, markSaved]);

  const overwriteDisk = useCallback(async () => {
    // Re-read to get the CURRENT disk hash, then force the write with it.
    const fresh = await api.files.read(wellId, path);
    const full = reconstructContent(fresh.content, fresh.body, bufferRef.current);
    setStatus({ kind: 'saving' });
    try {
      const result = await update.mutateAsync({ path, content: full, expectedHash: fresh.hash });
      markSaved(result.hash);
      setStatus({ kind: 'saved', at: Date.now() });
    } catch (err) {
      setStatus({
        kind: 'error',
        message: err instanceof Error ? err.message : 'Overwrite failed',
      });
    }
  }, [wellId, path, update, setStatus, markSaved]);

  const openMerge = useCallback(async () => {
    try {
      const fresh = await api.files.read(wellId, path);
      mergeDiskRef.current = fresh.body;
      // Keep the conflict status; flip the reactive flag so the view opens its
      // MergeDialog (Task 1.3 reads `mergeOpen` from the registry).
      patchView(key, { mergeOpen: true });
    } catch (err) {
      setStatus({
        kind: 'error',
        message: err instanceof Error ? err.message : 'Failed to read file',
      });
    }
  }, [wellId, path, key, patchView, setStatus]);

  const resolveMerge = useCallback(
    async (mergedBody: string) => {
      const fresh = await api.files.read(wellId, path);
      const full = reconstructContent(fresh.content, fresh.body, mergedBody);
      setStatus({ kind: 'saving' });
      try {
        const result = await update.mutateAsync({ path, content: full, expectedHash: fresh.hash });
        setBuffer(mergedBody);
        markSaved(result.hash);
        patchView(key, { mergeOpen: false });
        setStatus({ kind: 'saved', at: Date.now() });
      } catch (err) {
        setStatus({
          kind: 'error',
          message: err instanceof Error ? err.message : 'Merge save failed',
        });
      }
    },
    [wellId, path, key, update, patchView, setBuffer, setStatus, markSaved],
  );

  const insertTemplate = useCallback(
    (rendered: string) => {
      // The CodeMirror view lives in the FileEditor; it inserts at cursor and
      // calls back into onChange. The host just exposes onChange; cursor-aware
      // insertion is handled in FileEditor (see Task 1.3).
      const cleaned = rendered.replaceAll('{{cursor}}', '');
      const cur = bufferRef.current;
      onChange(cur ? `${cur}\n\n${cleaned}` : cleaned);
    },
    [onChange],
  );

  // overwriteDisk and resolveMerge close over `update` (useMutation), whose object
  // identity flips on every idle→pending→success transition. Read them via refs so
  // the registerActions effect below does NOT unregister/re-register on every save
  // cycle — same pattern as performSaveRef.
  const overwriteDiskRef = useRef(overwriteDisk);
  const resolveMergeRef = useRef(resolveMerge);
  useEffect(() => {
    overwriteDiskRef.current = overwriteDisk;
    resolveMergeRef.current = resolveMerge;
  });

  // Register imperative actions for the view to call. Every dep here is stable for
  // the host's lifetime, so this effect effectively re-runs only when `key` changes.
  useEffect(() => {
    registerActions(key, {
      onChange,
      onForceSave,
      reloadFromDisk: () => void reloadFromDisk(),
      overwriteDisk: () => void overwriteDiskRef.current(),
      openMerge: () => void openMerge(),
      resolveMerge: (mergedBody) => void resolveMergeRef.current(mergedBody),
      closeMerge: () => patchView(key, { mergeOpen: false }),
      insertTemplate,
      getMergeDisk: () => mergeDiskRef.current,
    });
    return () => unregisterActions(key);
  }, [key, onChange, onForceSave, reloadFromDisk, openMerge, insertTemplate, patchView]);

  return null;
}
