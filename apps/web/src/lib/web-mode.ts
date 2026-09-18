import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { api } from './api-client.ts';

/** True when running inside the Tauri desktop webview (vs a plain browser). */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

const TOKEN_KEY = 'mneme.webToken';

/**
 * Read a `#token=<hex>` fragment into localStorage and strip it from the URL (W7).
 *
 * The token still ONLY ever ENTERS the browser via this fragment (injected by the
 * desktop "Open in browser" button) — it is never logged or baked into served
 * HTML. We persist it in localStorage (not sessionStorage) so it survives new
 * tabs, reloads and reopens within the same browser profile; a bare URL,
 * bookmark or fresh tab can then still authenticate without re-running the
 * desktop button.
 */
export function bootstrapToken(): void {
  if (typeof window === 'undefined') return;
  const m = window.location.hash.match(/token=([a-f0-9]+)/i);
  const token = m?.[1];
  if (token) {
    localStorage.setItem(TOKEN_KEY, token);
    // Strip the fragment so the token isn't left in the URL/history.
    history.replaceState(null, '', window.location.pathname + window.location.search);
  }
}

export function getWebToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(TOKEN_KEY);
}

/** Drop the persisted token (e.g. after the server rejects it with a 401). */
export function clearWebToken(): void {
  if (typeof window === 'undefined') return;
  localStorage.removeItem(TOKEN_KEY);
}

/**
 * Decoupled signal that web access auth has expired. The api-client fires this
 * on a 401; a root overlay listens for it — a `window` CustomEvent avoids an
 * import cycle between the transport and the UI.
 */
export const WEB_AUTH_EXPIRED_EVENT = 'mneme:web-auth-expired';

export function notifyWebAuthExpired(): void {
  if (typeof window !== 'undefined') window.dispatchEvent(new Event(WEB_AUTH_EXPIRED_EVENT));
}

// ---------------------------------------------------------------------------
// TanStack Query hooks for web-mode status / lifecycle (W10)
// ---------------------------------------------------------------------------

export const WEB_MODE_KEY = ['web-mode', 'status'] as const;

export function useWebModeStatus() {
  return useQuery({
    queryKey: WEB_MODE_KEY,
    queryFn: api.webMode.status,
    // Only poll when inside Tauri — browser clients can't invoke Tauri commands.
    enabled: isTauri(),
    staleTime: 10_000,
    refetchInterval: 15_000,
  });
}

export function useEnableWebMode() {
  const qc = useQueryClient();
  return useMutation({
    // `lan: true` also allows other devices on this Wi-Fi (e.g. your phone) to connect.
    mutationFn: (lan: boolean) => api.webMode.enable(lan),
    onSuccess: (data) => {
      qc.setQueryData(WEB_MODE_KEY, data);
      toast.success(data.lanUrl ? 'Web access enabled for this network' : 'Web access enabled');
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to enable web access');
    },
  });
}

export function useDisableWebMode() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.webMode.disable,
    onSuccess: () => {
      // disable returns void; re-fetch to get the authoritative stopped state.
      qc.invalidateQueries({ queryKey: WEB_MODE_KEY });
      toast.success('Web access disabled');
    },
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to disable web access');
    },
  });
}

export function useOpenInBrowser() {
  return useMutation({
    mutationFn: api.webMode.openBrowser,
    onError: (err) => {
      toast.error(err instanceof Error ? err.message : 'Failed to open browser');
    },
  });
}
