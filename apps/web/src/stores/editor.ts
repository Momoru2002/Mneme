import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ViewMode = 'split' | 'editor' | 'preview';

export type RevealTarget = { docKey: string; line: number; column: number };

export type EditorState = {
  /** Transient: a (1-based) line/column the editor should scroll to + select, then clear. */
  revealTarget: RevealTarget | null;
  setRevealTarget: (t: RevealTarget | null) => void;
  /**
   * The view mode (editor / split / preview) — GLOBAL: one layout applied to
   * every file. Switching it (from the editor toolbar or Settings) affects all
   * files rather than diverging per-file, so the editing experience stays
   * consistent. Persisted across launches (EDITING-1).
   */
  defaultViewMode: ViewMode;
  /** Set the global view mode (Settings control). */
  setDefaultViewMode: (mode: ViewMode) => void;
};

export const useEditorStore = create<EditorState>()(
  persist(
    (set) => ({
      revealTarget: null,
      setRevealTarget: (revealTarget) => set({ revealTarget }),
      defaultViewMode: 'editor',
      setDefaultViewMode: (defaultViewMode) => set({ defaultViewMode }),
    }),
    {
      name: 'mneme.editor',
      // Restore the global view mode on relaunch.
      partialize: (state) => ({
        defaultViewMode: state.defaultViewMode,
      }),
    },
  ),
);

/** The (global) view mode applied to the open file. */
export function selectViewMode(state: EditorState): ViewMode {
  return state.defaultViewMode;
}

export function selectDefaultViewMode(state: EditorState): ViewMode {
  return state.defaultViewMode;
}
