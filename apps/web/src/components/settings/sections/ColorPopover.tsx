import { Pipette } from 'lucide-react';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { HexColorInput, HexColorPicker } from 'react-colorful';
import { normalizeHex } from '../../../lib/accent.ts';
import { Popover, PopoverContent, PopoverTrigger } from '../../ui/popover.tsx';

/**
 * A tidy in-app color picker: a swatch trigger that opens a popover with a
 * hue/saturation wheel + a hex field. Replaces the native `<input type="color">`,
 * which on macOS pops the OS-level NSColorPanel — a floating window that can't be
 * styled and sits detached from the Settings dialog.
 *
 * Dragging the wheel fires `onChange` dozens of times per second, so the two
 * concerns are split: `preview` repaints the live CSS var on every move (smooth),
 * while `commit` (which writes to SQLite over IPC) is debounced so a whole drag
 * collapses to a single write.
 */
export function ColorPopover({
  value,
  preview,
  commit,
  label,
}: {
  /** Current effective color (a 6-digit hex; the theme default when unset). */
  value: string;
  /**
   * Repaint a live preview on every move — must be cheap. Optional: omit it when
   * there's no cheap live target (e.g. a per-row swatch that only settles on
   * commit); the popover's own trigger swatch still tracks the draft live.
   */
  preview?: (hex: string) => void;
  /** Persist the choice. Debounced; one write per drag. */
  commit: (hex: string) => void;
  label: string;
}): React.JSX.Element {
  const [draft, setDraft] = useState(value);
  const pending = useRef<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Re-sync the wheel when `value` changes from outside (preset click, "Theme
  // default", a theme switch, our own commit landing). Such an external choice
  // SUPERSEDES anything this picker had queued, so cancel the pending debounced
  // write too — otherwise a stale trailing commit would resurrect the abandoned
  // color and silently clobber the user's last action. During an active drag
  // `value` is frozen (our commit is still pending), so this never fights the
  // user's pointer.
  useEffect(() => {
    if (timer.current) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    pending.current = null;
    setDraft(value);
  }, [value]);

  // Flush a pending write if we unmount mid-debounce (e.g. the dialog is closed
  // right after a pick) so the last color isn't dropped. `commit` is stable, so
  // setup runs once and cleanup runs on unmount.
  useEffect(() => {
    return () => {
      if (timer.current) clearTimeout(timer.current);
      if (pending.current != null) commit(pending.current);
    };
  }, [commit]);

  const handleChange = (raw: string) => {
    // The hex field can emit shorthand/uppercase; canonicalize before it reaches
    // the live CSS var, the persisted value, or the preset equality checks.
    const hex = normalizeHex(raw);
    setDraft(hex);
    preview?.(hex);
    pending.current = hex;
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      commit(hex);
      pending.current = null;
      timer.current = null;
    }, 140);
  };

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label={label}
          className="flex h-6 items-center gap-1 rounded-md border border-border px-1 hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <span
            className="h-4 w-4 rounded-sm border border-border"
            style={{ backgroundColor: draft }}
          />
          <Pipette className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-auto">
        <HexColorPicker color={draft} onChange={handleChange} />
        <div className="mt-3 flex items-center gap-2">
          <span className="text-xs text-muted-foreground">Hex</span>
          <HexColorInput
            color={draft}
            onChange={handleChange}
            prefixed
            aria-label={`${label} hex value`}
            className="h-8 w-28 rounded-md border border-border bg-background px-2 text-sm uppercase tabular-nums focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}
