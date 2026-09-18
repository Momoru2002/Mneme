import type { SearchOptions } from '@mneme/shared';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type SearchState = {
  options: SearchOptions;
  setOption: (key: keyof SearchOptions, value: boolean) => void;
  query: string;
  setQuery: (q: string) => void;
};

export const useSearchStore = create<SearchState>()(
  persist(
    (set, get) => ({
      options: { caseSensitive: false, wholeWord: false, regex: false },
      setOption: (key, value) => set({ options: { ...get().options, [key]: value } }),
      query: '',
      setQuery: (query) => set({ query }),
    }),
    {
      name: 'mneme.search',
      partialize: (s) => ({ options: s.options }),
    },
  ),
);
