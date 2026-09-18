import type React from 'react';
import { useEffect, useRef } from 'react';
import { useDocumentTitle } from '../../lib/use-document-title.ts';
import { useDocumentsStore } from '../../stores/documents.ts';
import { eachGroup, useWorkspaceStore } from '../../stores/workspace.ts';
import { DocumentHost } from './DocumentHost.tsx';
import { WorkspaceLayout } from './WorkspaceLayout.tsx';

type Props = { wellId: string | null };

/**
 * Phase-2 workspace shell: renders the split-tree layout (a single root group for
 * now; recursion arrives in Phase 3) and mounts exactly one `DocumentHost` per
 * unique open doc across the whole tree. The host owns the per-file edit engine;
 * this shell owns acquire-on-appear / release-on-disappear of every open doc and
 * the no-well empty state.
 */
export function EditorWorkbench({ wellId }: Props): React.JSX.Element {
  const root = useWorkspaceStore((s) => s.root);
  const activeGroupId = useWorkspaceStore((s) => s.activeGroupId);
  const acquire = useDocumentsStore((s) => s.acquire);
  const release = useDocumentsStore((s) => s.release);
  const migrate = useWorkspaceStore((s) => s.migrateLegacyOpenPath);

  // One-time migration: returning single-file users had their last file in the
  // legacy `mneme.editor.openPath`. Seed it as the first tab — but only when the
  // workspace is still a fresh empty root group (the action is idempotent), so a
  // returning multi-tab user (already-hydrated `mneme.workspace`) is a no-op.
  // biome-ignore lint/correctness/useExhaustiveDependencies: run once on mount per well — reads legacy localStorage and seeds only if the workspace is still empty.
  useEffect(() => {
    if (!wellId) return;
    try {
      const raw = localStorage.getItem('mneme.editor');
      if (!raw) return;
      const parsed = JSON.parse(raw) as { state?: { openPath?: string | null } };
      const legacy = parsed?.state?.openPath;
      if (legacy) migrate(wellId, legacy);
    } catch {
      /* ignore malformed legacy state */
    }
  }, [wellId]);

  // Compute the set of unique open docs across the tree.
  const open: { wellId: string; path: string }[] = [];
  const seen = new Set<string>();
  eachGroup(root, (g) => {
    for (const t of g.tabs) {
      const k = `${t.wellId}::${t.path}`;
      if (!seen.has(k)) {
        seen.add(k);
        open.push(t);
      }
    }
  });

  // Title from the active group's active tab.
  let title: string | null = null;
  eachGroup(root, (g) => {
    if (g.id === activeGroupId && g.activeTab !== null) {
      const t = g.tabs[g.activeTab];
      if (t) title = t.path.split('/').pop() ?? null;
    }
  });
  useDocumentTitle(title);

  // Acquire on first appearance, release on disappearance. The effect runs every
  // render and diffs the previous open-set against the next one, so there is no
  // double-acquire on mount and each tab close releases exactly once.
  const prevRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    const next = new Set(open.map((t) => `${t.wellId}::${t.path}`));
    for (const t of open) {
      if (!prevRef.current.has(`${t.wellId}::${t.path}`)) acquire(t.wellId, t.path);
    }
    for (const k of prevRef.current) {
      if (!next.has(k)) {
        // Split on the first '::'; the remainder is the path (defensive against a
        // '::' inside a filename, which POSIX paths never contain in practice).
        const sep = k.indexOf('::');
        void release(k.slice(0, sep), k.slice(sep + 2));
      }
    }
    prevRef.current = next;
  });

  // Release every still-acquired doc when the workbench unmounts (route change),
  // so refcounts don't leak across navigation. Mount-only (empty-ish deps) so it
  // does NOT release+reacquire on every render — that's the diff effect's job.
  useEffect(() => {
    return () => {
      for (const k of prevRef.current) {
        const sep = k.indexOf('::');
        void release(k.slice(0, sep), k.slice(sep + 2));
      }
      prevRef.current = new Set();
    };
  }, [release]);

  if (!wellId) return <EmptyState message="Select or add a well to view files." />;

  return (
    <>
      {open.map((t) => (
        <DocumentHost key={`${t.wellId}::${t.path}`} wellId={t.wellId} path={t.path} />
      ))}
      <div className="h-full w-full">
        <WorkspaceLayout node={root} />
      </div>
    </>
  );
}

export function EmptyState({
  message,
  error,
}: { message: string; error?: boolean }): React.JSX.Element {
  return (
    <div className="flex h-full items-center justify-center p-6">
      <p className={error ? 'text-sm text-destructive' : 'text-sm text-muted-foreground italic'}>
        {message}
      </p>
    </div>
  );
}
