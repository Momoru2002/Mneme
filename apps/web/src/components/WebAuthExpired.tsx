import { Globe } from 'lucide-react';
import type React from 'react';
import { useEffect, useState } from 'react';
import { WEB_AUTH_EXPIRED_EVENT, getWebToken, isTauri } from '../lib/web-mode.ts';
import { Button } from './ui/button.tsx';

/**
 * Root overlay for web mode: when the access token is missing or rejected (401),
 * show a clear, actionable full-screen message instead of a silently empty UI.
 * Inert inside the Tauri desktop webview (renders nothing there).
 */
export function WebAuthExpired(): React.JSX.Element | null {
  // Opened with no token at all (bare URL / bookmark / fresh tab) → expired.
  const [expired, setExpired] = useState(() => !isTauri() && !getWebToken());

  useEffect(() => {
    if (isTauri()) return;
    const onExpired = (): void => setExpired(true);
    window.addEventListener(WEB_AUTH_EXPIRED_EVENT, onExpired);
    return () => window.removeEventListener(WEB_AUTH_EXPIRED_EVENT, onExpired);
  }, []);

  if (isTauri() || !expired) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      // biome-ignore lint/a11y/useSemanticElements: a fixed full-screen overlay backdrop, not a native <dialog> (no showModal lifecycle); role="dialog" carries the modal semantics.
      role="dialog"
      aria-modal="true"
      aria-labelledby="web-auth-expired-title"
    >
      <div className="flex w-full max-w-md flex-col items-center gap-4 rounded-lg border border-border bg-card p-6 text-center shadow-lg">
        <span className="flex h-12 w-12 items-center justify-center rounded-full bg-muted">
          <Globe className="h-6 w-6 text-muted-foreground" aria-hidden="true" />
        </span>
        <h2 id="web-auth-expired-title" className="text-lg font-semibold tracking-tight">
          Web access link expired
        </h2>
        <p className="text-sm text-muted-foreground">
          Your access token is missing or expired. In the Mneme desktop app, open Settings → Web
          Access → Open in browser to get a fresh link (it opens a new tab).
        </p>
        <Button variant="outline" size="sm" onClick={() => window.location.reload()}>
          Reload
        </Button>
      </div>
    </div>
  );
}
