import { create } from 'zustand';

/**
 * Transient UI coordination that doesn't belong to any single component:
 * - the activity rail's "Files"/"Search" buttons pick which view the sidebar
 *   shows (`sidebarView`) — the shell renders FilesPanel or SearchPanel for it;
 * - the command palette's "Open Settings" asks the rail to open the Settings dialog;
 * - the file-tree sidebar's visibility is owned here so the rail's "Files" button
 *   and the sidebar's own collapse button toggle the same panel (VS Code pattern).
 * The *Nonce fields are monotonic counters so the listener reacts even to repeats.
 */
export type SidebarView = 'files' | 'search';

export type UiState = {
  /** Which view the sidebar shows (Files tree vs. Search). */
  sidebarView: SidebarView;
  setSidebarView: (v: SidebarView) => void;
  settingsOpenNonce: number;
  /** A category id to land on when settings opens next (e.g. 'wells'), or null. */
  settingsTarget: string | null;
  requestSettingsOpen: (target?: string) => void;
  clearSettingsTarget: () => void;
  /** Monotonic counter; a bump asks the global New File dialog to open. */
  newFileNonce: number;
  requestNewFile: () => void;
  /** Whether the file-tree sidebar is hidden (the rail stays visible regardless). */
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
};

export const useUiStore = create<UiState>((set, get) => ({
  sidebarView: 'files',
  setSidebarView: (sidebarView) => set({ sidebarView }),
  settingsOpenNonce: 0,
  settingsTarget: null,
  requestSettingsOpen: (target) =>
    set({ settingsOpenNonce: get().settingsOpenNonce + 1, settingsTarget: target ?? null }),
  clearSettingsTarget: () => set({ settingsTarget: null }),
  newFileNonce: 0,
  requestNewFile: () => set({ newFileNonce: get().newFileNonce + 1 }),
  sidebarCollapsed: false,
  toggleSidebar: () => set({ sidebarCollapsed: !get().sidebarCollapsed }),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
}));
