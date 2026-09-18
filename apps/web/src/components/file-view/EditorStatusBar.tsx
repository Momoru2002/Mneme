import { CheckCircle2, Loader2, OctagonAlert } from 'lucide-react';
import type React from 'react';
import { cn } from '../../lib/cn.ts';
import type { SaveStatus } from '../../lib/save-status.ts';
export type { SaveStatus };

/** The autosave status text for each kind. */
export function saveStatusLabel(status: SaveStatus): string {
  switch (status.kind) {
    case 'saving':
      return 'Saving…';
    case 'dirty':
      return 'Unsaved changes…';
    case 'saved':
      return 'Saved';
    case 'conflict':
      return 'Conflict';
    case 'error':
      return 'Save failed';
    default:
      return 'All changes saved';
  }
}

function SaveIndicator({ status }: { status: SaveStatus }): React.JSX.Element {
  const label = saveStatusLabel(status);
  let icon: React.JSX.Element | null = null;
  let color = 'text-muted-foreground';
  if (status.kind === 'saving') {
    icon = <Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />;
    color = 'text-mneme-cyan';
  } else if (status.kind === 'dirty') {
    color = 'text-mneme-warning';
  } else if (status.kind === 'saved') {
    icon = <CheckCircle2 aria-hidden="true" className="h-3.5 w-3.5" />;
    color = 'text-mneme-success';
  } else if (status.kind === 'conflict' || status.kind === 'error') {
    icon = <OctagonAlert aria-hidden="true" className="h-3.5 w-3.5" />;
    color = 'text-mneme-danger';
  }
  return (
    // biome-ignore lint/a11y/useSemanticElements: intentional ARIA live region for save status; <output> implies form-output semantics that don't apply here.
    <span
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className={cn('flex items-center gap-1', color)}
      title={status.kind === 'error' ? status.message : undefined}
    >
      {icon}
      {label}
    </span>
  );
}

/** Slim bottom status bar: autosave state. */
export function EditorStatusBar({ status }: { status: SaveStatus }): React.JSX.Element {
  return (
    <footer className="flex items-center gap-3 border-t border-border bg-card px-4 py-1 text-xs text-muted-foreground">
      <SaveIndicator status={status} />
    </footer>
  );
}
