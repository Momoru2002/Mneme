import { DEFAULT_PREFS, type UserPrefsPatch } from '@mneme/shared';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';
import { toast } from 'sonner';
import { applyAccent, applyGold } from './accent.ts';
import { api } from './api-client.ts';
import { setTheme } from './theme.ts';

export const SETTINGS_KEY = ['settings'] as const;

export function useSettings() {
  return useQuery({
    queryKey: SETTINGS_KEY,
    queryFn: api.settings.get,
    retry: (failureCount) => failureCount < 1,
    staleTime: 60_000,
  });
}

export function usePrefs(): typeof DEFAULT_PREFS {
  const s = useSettings();
  return s.data?.prefs ?? DEFAULT_PREFS;
}

/** Apply a theme live (DOM + localStorage) AND persist it to SQLite, quietly. */
export function useApplyTheme(): (name: string) => void {
  const qc = useQueryClient();
  return useCallback(
    (name: string) => {
      setTheme(name);
      // Cancel any in-flight settings GET so its (now-stale) result can't clobber
      // the theme write we're about to commit to the cache.
      void qc.cancelQueries({ queryKey: SETTINGS_KEY });
      void api.settings
        .update({ theme: name })
        .then((data) => qc.setQueryData(SETTINGS_KEY, data))
        .catch(() => {});
    },
    [qc],
  );
}

/** Apply an accent override live AND persist it (null clears). */
export function useApplyAccent(): (color: string | null) => void {
  const qc = useQueryClient();
  return useCallback(
    (color: string | null) => {
      applyAccent(color);
      void qc.cancelQueries({ queryKey: SETTINGS_KEY });
      void api.settings
        .update({ accentColor: color })
        .then((data) => qc.setQueryData(SETTINGS_KEY, data))
        .catch(() => {});
    },
    [qc],
  );
}

/** Apply a secondary (gold) accent override live AND persist it (null clears). */
export function useApplyGold(): (color: string | null) => void {
  const qc = useQueryClient();
  return useCallback(
    (color: string | null) => {
      applyGold(color);
      void qc.cancelQueries({ queryKey: SETTINGS_KEY });
      void api.settings
        .update({ goldColor: color })
        .then((data) => qc.setQueryData(SETTINGS_KEY, data))
        .catch(() => {});
    },
    [qc],
  );
}

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: UserPrefsPatch) => api.settings.update(patch),
    onSuccess: (data) => {
      qc.setQueryData(SETTINGS_KEY, data);
      toast.success('Settings saved');
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed'),
  });
}
