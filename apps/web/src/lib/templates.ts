import type { ApplyTemplateInput, TemplateCreateInput, TemplateUpdateInput } from '@mneme/shared';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { api } from './api-client.ts';

export const TEMPLATES_KEY = ['templates'] as const;
export const templateKey = (name: string) => ['template', name] as const;

export function useTemplates() {
  return useQuery({
    queryKey: TEMPLATES_KEY,
    queryFn: api.templates.list,
    retry: (failureCount) => failureCount < 1,
    staleTime: 30_000,
  });
}

export function useTemplate(name: string | null | undefined) {
  return useQuery({
    queryKey: name ? templateKey(name) : ['template', 'idle'],
    queryFn: () => {
      if (!name) throw new Error('No template selected');
      return api.templates.get(name);
    },
    enabled: Boolean(name),
    staleTime: 0,
  });
}

export function useCreateTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: TemplateCreateInput) => api.templates.create(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: TEMPLATES_KEY });
      toast.success('Template created');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed'),
  });
}

export function useUpdateTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, input }: { name: string; input: TemplateUpdateInput }) =>
      api.templates.update(name, input),
    onSuccess: (data) => {
      qc.invalidateQueries({ queryKey: TEMPLATES_KEY });
      qc.setQueryData(templateKey(data.name), data);
      toast.success('Template saved');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed'),
  });
}

export function useDeleteTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => api.templates.remove(name),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: TEMPLATES_KEY });
      toast.success('Template deleted');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed'),
  });
}

export function useDuplicateTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => api.templates.duplicate(name),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: TEMPLATES_KEY });
      toast.success('Template duplicated');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed'),
  });
}

export function useApplyTemplate() {
  return useMutation({
    mutationFn: ({ name, input }: { name: string; input: ApplyTemplateInput }) =>
      api.templates.apply(name, input),
  });
}

export function useImportDefaults() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.templates.importDefaults(),
    onSuccess: (data) => {
      qc.invalidateQueries({ queryKey: TEMPLATES_KEY });
      if (data.created.length === 0) {
        toast.info('Default templates already exist');
      } else {
        toast.success(`Imported ${data.created.length} default templates`);
      }
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed'),
  });
}
