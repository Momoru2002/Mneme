import { useQuery } from '@tanstack/react-query';
import { api } from './api-client.ts';

export function treeKey(wellId: string, path: string): readonly unknown[] {
  return ['tree', wellId, path] as const;
}

export function useTree(wellId: string | null | undefined, path = '') {
  return useQuery({
    queryKey: wellId ? treeKey(wellId, path) : ['tree', 'idle'],
    queryFn: () => {
      if (!wellId) throw new Error('No well');
      return api.tree.list(wellId, path);
    },
    enabled: Boolean(wellId),
    retry: (failureCount) => failureCount < 1,
    staleTime: 15_000,
  });
}
