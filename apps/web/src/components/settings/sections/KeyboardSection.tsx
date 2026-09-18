import type React from 'react';
import { isMacPlatform, keyGlyph } from '../../../lib/platform.ts';

// Shortcuts are stored as symbolic tokens (Mod/Alt/Shift/letters); glyphs are
// resolved per-OS at render (⌘/⌥/⇧ on macOS, Ctrl/Alt/Shift elsewhere).
type Shortcut = { keys: string[]; action: string };
const GROUPS: { group: string; items: Shortcut[] }[] = [
  {
    group: 'Editor',
    items: [
      { keys: ['Mod', 'S'], action: 'Save now' },
      { keys: ['Mod', 'Alt', 'N'], action: 'New file' },
      { keys: ['Mod', 'Alt', 'W'], action: 'Close all tabs' },
    ],
  },
  {
    group: 'Navigation',
    items: [
      { keys: ['Mod', 'K'], action: 'Command palette' },
      { keys: ['Mod', 'P'], action: 'Quick-open file' },
      { keys: ['Mod', 'Shift', 'F'], action: 'Search' },
    ],
  },
  {
    group: 'Window',
    items: [{ keys: ['Alt', 'Mod', 'P'], action: 'Toggle Float (always-on-top)' }],
  },
  { group: 'General', items: [{ keys: ['Esc'], action: 'Close dialog / palette' }] },
];

export function KeyboardSection(): React.JSX.Element {
  const isMac = isMacPlatform();
  return (
    <div className="flex flex-col gap-4">
      {GROUPS.map((g) => (
        <div key={g.group} className="flex flex-col gap-1">
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {g.group}
          </p>
          {g.items.map((s) => (
            <div
              key={s.action}
              className="flex items-center justify-between rounded-md px-2 py-1 text-sm"
            >
              <span>{s.action}</span>
              <span className="flex gap-1">
                {s.keys.map((k) => (
                  <kbd
                    key={k}
                    className="rounded border border-border bg-muted px-1.5 font-mono text-xs"
                  >
                    {keyGlyph(k, isMac)}
                  </kbd>
                ))}
              </span>
            </div>
          ))}
        </div>
      ))}
      <p className="text-xs text-muted-foreground">
        Shortcuts are fixed in this version; remapping isn't available yet.
      </p>
    </div>
  );
}
