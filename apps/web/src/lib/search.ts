import type { SearchOptions } from '@mneme/shared';
import { useQuery } from '@tanstack/react-query';
import { api } from './api-client.ts';

export function searchKey(
  wellId: string,
  q: string,
  includeContent: boolean,
  options: SearchOptions,
): readonly unknown[] {
  return ['search', wellId, q, includeContent, options] as const;
}

export function useSearch(
  wellId: string | null | undefined,
  query: string,
  includeContent = true,
  options: SearchOptions = { caseSensitive: false, wholeWord: false, regex: false },
) {
  const trimmed = query.trim();
  return useQuery({
    queryKey:
      wellId && trimmed ? searchKey(wellId, trimmed, includeContent, options) : ['search', 'idle'],
    queryFn: () => {
      if (!wellId || !trimmed) throw new Error('No query');
      return api.search.query(wellId, trimmed, includeContent, options);
    },
    enabled: Boolean(wellId && trimmed.length >= 2),
    retry: (failureCount) => failureCount < 1,
    staleTime: 10_000,
  });
}
