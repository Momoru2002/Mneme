import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type WindowState = {
  /** Whether the desktop window is pinned above other apps (Float). */
  alwaysOnTop: boolean;
  setAlwaysOnTop: (v: boolean) => void;
};

export const useWindowStore = create<WindowState>()(
  persist(
    (set) => ({
      alwaysOnTop: false,
      setAlwaysOnTop: (alwaysOnTop) => set({ alwaysOnTop }),
    }),
    { name: 'mneme.window' },
  ),
);
