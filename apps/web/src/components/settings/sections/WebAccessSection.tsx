import { ExternalLink, Globe } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { toast } from 'sonner';
import {
  useDisableWebMode,
  useEnableWebMode,
  useOpenInBrowser,
  useWebModeStatus,
} from '../../../lib/web-mode.ts';
import { Button } from '../../ui/button.tsx';
import { Field } from '../controls.tsx';

export function WebAccessSection(): React.JSX.Element {
  const status = useWebModeStatus();
  const enable = useEnableWebMode();
  const disable = useDisableWebMode();
  const openBrowser = useOpenInBrowser();
  const [allowLan, setAllowLan] = useState(false);

  const running = status.data?.running ?? false;
  const url = status.data?.url ?? null;
  const lanUrl = status.data?.lanUrl ?? null;

  async function copyLanUrl(): Promise<void> {
    if (!lanUrl) return;
    try {
      await navigator.clipboard.writeText(lanUrl);
      toast.success('Link copied');
    } catch {
      toast.error('Could not copy the link');
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-muted-foreground">
        Use Mneme in a browser tab against this running desktop app. Localhost only — keep this
        desktop app open while using the browser view.
      </p>

      <Field label="Status">
        <div className="flex items-center gap-2">
          <span
            className={`h-2 w-2 shrink-0 rounded-full ${running ? 'bg-mneme-cyan' : 'bg-muted-foreground/40'}`}
            aria-hidden="true"
          />
          <span className="text-sm">
            {status.isPending ? 'Checking…' : running ? 'Running' : 'Stopped'}
          </span>
        </div>
      </Field>

      {running && url && (
        <Field label="URL">
          <div className="flex items-center gap-2">
            <p className="flex-1 truncate text-sm text-muted-foreground">
              Running at <span className="font-mono">{url.replace(/^https?:\/\//, '')}</span>
            </p>
            <Button
              size="sm"
              variant="outline"
              onClick={() => openBrowser.mutate()}
              disabled={openBrowser.isPending}
              title="Open in browser"
            >
              <ExternalLink className="h-3.5 w-3.5" />
              Open in browser
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Open the browser view with this button — the link carries a one-time access token.
            Copying or bookmarking the address alone won't sign you in.
          </p>
          <p className="text-xs text-muted-foreground">
            {lanUrl
              ? 'Also reachable from other devices on this network (see below).'
              : 'Accessible only from this machine (127.0.0.1). Devices on your local network cannot reach it.'}
          </p>
        </Field>
      )}

      {running && lanUrl && (
        <Field label="Network link">
          <div className="flex items-center gap-2">
            <p className="flex-1 truncate text-sm text-muted-foreground">
              From your phone or another device on this Wi-Fi:{' '}
              <span className="font-mono">{lanUrl.replace(/^https?:\/\//, '')}</span>
            </p>
            <Button size="sm" variant="outline" onClick={copyLanUrl} title="Copy link">
              Copy link
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Open this address on the other device's browser, then enter the token shown by "Open
            in browser" on this desktop app.
          </p>
        </Field>
      )}

      <Field label="Control">
        {running ? (
          <div className="flex flex-col gap-2">
            <div>
              <Button
                size="sm"
                variant="outline"
                onClick={() => disable.mutate()}
                disabled={disable.isPending}
              >
                {disable.isPending ? 'Stopping…' : 'Stop web access'}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Stopping will disconnect any open browser tabs.
            </p>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={allowLan}
                onChange={(e) => setAllowLan(e.target.checked)}
                className="h-3.5 w-3.5"
              />
              Allow other devices on this network (e.g. your phone)
            </label>
            <div>
              <Button
                size="sm"
                onClick={() => enable.mutate(allowLan)}
                disabled={enable.isPending || status.isPending}
              >
                <Globe className="h-3.5 w-3.5" />
                {enable.isPending ? 'Starting…' : 'Enable web access'}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Starts a local server and opens a browser tab with a one-time access token.
              {allowLan &&
                ' Other devices on this Wi-Fi will be able to reach it too, using the same token.'}
            </p>
          </div>
        )}
      </Field>
    </div>
  );
}
