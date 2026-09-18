import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type FileTreeState = {
  /** Expanded folder paths, scoped per well (paths are well-relative). Persisted. */
  expandedByWell: Record<string, string[]>;
  /** Transient: the path currently hovered, for the active-branch guide highlight. Not persisted. */
  hoverPath: string | null;
  setExpanded: (wellId: string, path: string, expanded: boolean) => void;
  toggleExpanded: (wellId: string, path: string) => void;
  /** Ensure each given folder path is expanded (reveals the open file). Additive — never collapses. */
  expandAncestors: (wellId: string, paths: string[]) => void;
  collapseAll: (wellId: string) => void;
  setHoverPath: (path: string | null) => void;
};

/** Ancestor folder paths of a node path: "a/b/c.md" -> ["a", "a/b"]; "x.md" -> []. */
export function ancestorsOf(nodePath: string): string[] {
  const parts = nodePath.split('/');
  const out: string[] = [];
  for (let i = 1; i < parts.length; i++) {
    out.push(parts.slice(0, i).join('/'));
  }
  return out;
}

/** Whether `target` is the folder itself or lives inside it (path-segment safe). */
export function insideOrEq(folderPath: string, target: string | null): boolean {
  if (target == null) return false;
  return target === folderPath || target.startsWith(`${folderPath}/`);
}

/** Persisted slice — only the expand state survives reloads, never the transient hover. */
export function partializeFileTree(s: FileTreeState): Pick<FileTreeState, 'expandedByWell'> {
  return { expandedByWell: s.expandedByWell };
}

export const useFileTreeStore = create<FileTreeState>()(
  persist(
    (set, get) => ({
      expandedByWell: {},
      hoverPath: null,
      setExpanded: (wellId, path, expanded) =>
        set((s) => {
          const cur = s.expandedByWell[wellId] ?? [];
          const has = cur.includes(path);
          if (expanded === has) return s;
          const next = expanded ? [...cur, path] : cur.filter((p) => p !== path);
          return { expandedByWell: { ...s.expandedByWell, [wellId]: next } };
        }),
      toggleExpanded: (wellId, path) => {
        const cur = get().expandedByWell[wellId] ?? [];
        get().setExpanded(wellId, path, !cur.includes(path));
      },
      expandAncestors: (wellId, paths) =>
        set((s) => {
          const cur = s.expandedByWell[wellId] ?? [];
          const merged = Array.from(new Set([...cur, ...paths]));
          if (merged.length === cur.length) return s;
          return { expandedByWell: { ...s.expandedByWell, [wellId]: merged } };
        }),
      collapseAll: (wellId) =>
        set((s) => {
          if ((s.expandedByWell[wellId] ?? []).length === 0) return s;
          return { expandedByWell: { ...s.expandedByWell, [wellId]: [] } };
        }),
      setHoverPath: (hoverPath) => set({ hoverPath }),
    }),
    {
      name: 'mneme.fileTree',
      partialize: partializeFileTree,
    },
  ),
);
