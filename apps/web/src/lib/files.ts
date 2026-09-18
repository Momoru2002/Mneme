import type {
  DuplicateInput,
  FileCreateInput,
  FileUpdateInput,
  MoveInput,
  RenameInput,
} from '@mneme/shared';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { ApiError, api } from './api-client.ts';

export function fileKey(wellId: string, path: string): readonly unknown[] {
  return ['file', wellId, path] as const;
}

export function treeRootKey(wellId: string): readonly unknown[] {
  return ['tree', wellId] as const;
}

export function useFileContent(wellId: string | null | undefined, path: string | null | undefined) {
  return useQuery({
    queryKey: wellId && path ? fileKey(wellId, path) : ['file', 'idle'],
    queryFn: () => {
      if (!wellId || !path) throw new Error('No file selected');
      return api.files.read(wellId, path);
    },
    enabled: Boolean(wellId && path),
    retry: (failureCount, err) => {
      if (err instanceof ApiError && err.code === 'not_found') return false;
      return failureCount < 1;
    },
    staleTime: 0,
  });
}

function invalidateAll(qc: ReturnType<typeof useQueryClient>, wellId: string) {
  qc.invalidateQueries({ queryKey: treeRootKey(wellId) });
  qc.invalidateQueries({ queryKey: ['file', wellId] });
}

export function useCreateFile(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: FileCreateInput) => {
      if (!wellId) throw new Error('No well');
      return api.files.create(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidateAll(qc, wellId);
      toast.success('File created');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to create file'),
  });
}

export function useUpdateFile(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: FileUpdateInput) => {
      if (!wellId) throw new Error('No well');
      return api.files.update(wellId, input);
    },
    onSuccess: (data) => {
      if (!wellId) return;
      // Update cached file content's mtime/size/hash optimistically
      qc.setQueryData(fileKey(wellId, data.path), (prev: unknown) =>
        prev && typeof prev === 'object'
          ? {
              ...(prev as Record<string, unknown>),
              size: data.size,
              mtime: data.mtime,
              hash: data.hash,
            }
          : prev,
      );
      qc.invalidateQueries({ queryKey: treeRootKey(wellId) });
      // A successful write dirties the well's git working tree — refresh the
      // version-history status indicator (no-op when the well isn't enabled).
      qc.invalidateQueries({ queryKey: ['git', 'status', wellId] });
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to save'),
  });
}

export function useDeleteFile(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (path: string) => {
      if (!wellId) throw new Error('No well');
      return api.files.remove(wellId, path);
    },
    onSuccess: () => {
      if (wellId) invalidateAll(qc, wellId);
      toast.success('File deleted');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to delete'),
  });
}

export function useRenameFile(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: RenameInput) => {
      if (!wellId) throw new Error('No well');
      return api.files.rename(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidateAll(qc, wellId);
      toast.success('Renamed');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to rename'),
  });
}

export function useMoveFile(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: MoveInput) => {
      if (!wellId) throw new Error('No well');
      return api.files.move(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidateAll(qc, wellId);
      toast.success('Moved');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to move'),
  });
}

export function useDuplicateFile(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: DuplicateInput) => {
      if (!wellId) throw new Error('No well');
      return api.files.duplicate(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidateAll(qc, wellId);
      toast.success('Duplicated');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to duplicate'),
  });
}
