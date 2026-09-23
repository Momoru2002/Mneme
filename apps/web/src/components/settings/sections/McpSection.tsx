import { Copy } from 'lucide-react';
import type React from 'react';
import { toast } from 'sonner';
import { claudeCodeCommand, claudeDesktopConfig, useMcpBinaryInfo } from '../../../lib/mcp.ts';
import { Button } from '../../ui/button.tsx';
import { Field } from '../controls.tsx';

async function copy(text: string, what: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(`${what} copied`);
  } catch {
    toast.error(`Could not copy ${what.toLowerCase()}`);
  }
}

function CopyBlock({ label, text }: { label: string; text: string }): React.JSX.Element {
  return (
    <div className="flex flex-col gap-1">
      <pre className="overflow-x-auto rounded-md border border-border bg-background p-2 font-mono text-xs">
        {text}
      </pre>
      <div>
        <Button size="sm" variant="outline" onClick={() => copy(text, label)}>
          <Copy className="h-3.5 w-3.5" />
          Copy {label.toLowerCase()}
        </Button>
      </div>
    </div>
  );
}

export function McpSection(): React.JSX.Element {
  const info = useMcpBinaryInfo();
  const path = info.data?.path ?? null;

  return (
    <div className="flex flex-col gap-6">
      <p className="text-sm text-muted-foreground">
        Let an AI assistant (Claude Desktop, Claude Code, or any MCP client) read and write your
        Wells through the bundled <span className="font-mono">mneme-mcp</span> server. It talks to
        the assistant over stdio only — nothing is exposed on the network.
      </p>

      {info.isPending ? (
        <p className="text-sm text-muted-foreground">Looking for the bundled server…</p>
      ) : path === null ? (
        <p className="text-sm text-muted-foreground">
          The bundled server was not found next to this app (normal for a development build). Build
          it with <span className="font-mono">cargo build --release --bin mneme-mcp</span> — see the
          README.
        </p>
      ) : (
        <>
          <Field label="Server location">
            <p className="break-all font-mono text-xs text-muted-foreground">{path}</p>
            {info.data?.ephemeral && (
              <p className="text-xs text-muted-foreground">
                This is an AppImage, so this path changes every time Mneme starts. Use the .deb or
                .rpm package — or copy <span className="font-mono">mneme-mcp</span> somewhere
                permanent — before saving it in an assistant's config.
              </p>
            )}
          </Field>

          <Field label="Claude Code">
            <CopyBlock label="Command" text={claudeCodeCommand(path)} />
          </Field>

          <Field label="Claude Desktop">
            <p className="text-xs text-muted-foreground">
              Add this to <span className="font-mono">claude_desktop_config.json</span> (Settings →
              Developer → Edit Config), keeping any keys already there, then fully quit and reopen
              Claude Desktop.
            </p>
            <CopyBlock label="Config" text={claudeDesktopConfig(path)} />
          </Field>
        </>
      )}
    </div>
  );
}
