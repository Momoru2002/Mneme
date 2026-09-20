import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { api } from './api-client.ts';
import { isTauri } from './web-mode.ts';

// ---------------------------------------------------------------------------
// TanStack Query hooks for the app-lock. Tauri-only: the app-lock exists to
// protect the desktop app (and, transitively, anyone holding a Web Mode
// token) — there is no unlock path over the web-mode HTTP transport by
// design, so these are never called (and `enabled: isTauri()` never fires)
// outside the desktop webview.
// ---------------------------------------------------------------------------

export const AUTH_STATUS_KEY = ['auth', 'status'] as const;

export function useAuthStatus() {
  return useQuery({
    queryKey: AUTH_STATUS_KEY,
    queryFn: api.auth.status,
    enabled: isTauri(),
    staleTime: 0,
    // The lock can change from outside this query's own mutations too
    // (e.g. "Lock now" from a native menu item) — poll gently so the UI
    // reliably shows a fresh lock screen rather than a stale unlocked one.
    refetchInterval: 5_000,
  });
}

export function useSetPassword() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (password: string) => api.auth.setPassword(password),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AUTH_STATUS_KEY });
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to set password');
    },
  });
}

export function useUnlock() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (password: string) => api.auth.unlock(password),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AUTH_STATUS_KEY });
    },
    // Deliberately no error toast here — the unlock form shows a wrong-password
    // message inline instead of a corner toast, since it's the only thing on
    // screen at that point.
  });
}

export function useLockNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.auth.lock,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AUTH_STATUS_KEY });
    },
  });
}

export function useChangePassword() {
  return useMutation({
    mutationFn: ({
      currentPassword,
      newPassword,
    }: {
      currentPassword: string;
      newPassword: string;
    }) => api.auth.changePassword(currentPassword, newPassword),
    onSuccess: () => {
      toast.success('Password changed');
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to change password');
    },
  });
}
