import type { AddWellInput, UpdateWellInput, WellListResponse } from '@mneme/shared';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { api } from './api-client.ts';

export const WELLS_KEY = ['wells'] as const;
export const HOST_HOME_KEY = ['wells', 'host-home'] as const;

export function useWells() {
  return useQuery({
    queryKey: WELLS_KEY,
    queryFn: api.wells.list,
    retry: (failureCount) => failureCount < 1,
    staleTime: 30_000,
  });
}

export function useHostHome() {
  return useQuery({
    queryKey: HOST_HOME_KEY,
    queryFn: api.wells.hostHome,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useAddWell() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: AddWellInput) => api.wells.add(input),
    onSuccess: (well) => {
      qc.invalidateQueries({ queryKey: WELLS_KEY });
      toast.success(`Well "${well.name}" added`);
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to add well');
    },
  });
}

export function useUpdateWell() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: UpdateWellInput }) =>
      api.wells.update(id, patch),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: WELLS_KEY });
      toast.success('Well updated');
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to update well');
    },
  });
}

export function useRemoveWell() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.wells.remove(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: WELLS_KEY });
      toast.success('Well removed from registry');
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to remove well');
    },
  });
}

export function useActivateWell() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.wells.activate(id),
    onSuccess: (_, wellId) => {
      // Optimistic patch: bump activeWellId immediately
      qc.setQueryData<WellListResponse>(WELLS_KEY, (prev) =>
        prev ? { ...prev, activeWellId: wellId } : prev,
      );
      qc.invalidateQueries({ queryKey: WELLS_KEY });
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to activate well');
    },
  });
}
