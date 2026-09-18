import type { FolderCreateInput, MoveInput, RenameInput } from '@mneme/shared';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { api } from './api-client.ts';
import { treeRootKey } from './files.ts';

function invalidate(qc: ReturnType<typeof useQueryClient>, wellId: string) {
  qc.invalidateQueries({ queryKey: treeRootKey(wellId) });
}

export function useCreateFolder(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: FolderCreateInput) => {
      if (!wellId) throw new Error('No well');
      return api.folders.create(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidate(qc, wellId);
      toast.success('Folder created');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to create folder'),
  });
}

export function useDeleteFolder(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (path: string) => {
      if (!wellId) throw new Error('No well');
      return api.folders.remove(wellId, path, true);
    },
    onSuccess: () => {
      if (wellId) invalidate(qc, wellId);
      toast.success('Folder deleted');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to delete folder'),
  });
}

export function useRenameFolder(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: RenameInput) => {
      if (!wellId) throw new Error('No well');
      return api.folders.rename(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidate(qc, wellId);
      toast.success('Renamed');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to rename folder'),
  });
}

export function useMoveFolder(wellId: string | null | undefined) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: MoveInput) => {
      if (!wellId) throw new Error('No well');
      return api.folders.move(wellId, input);
    },
    onSuccess: () => {
      if (wellId) invalidate(qc, wellId);
      toast.success('Moved');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to move folder'),
  });
}
