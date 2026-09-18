import type React from 'react';
import { applyAccent, applyGold } from '../../../lib/accent.ts';
import { cn } from '../../../lib/cn.ts';
import { useApplyAccent, useApplyGold, useApplyTheme, usePrefs } from '../../../lib/settings.ts';
import { useTheme } from '../../../lib/theme.ts';
import { Input } from '../../ui/input.tsx';
import { Field, type PrefsGet, type PrefsSet } from '../controls.tsx';
import { ColorPopover } from './ColorPopover.tsx';

export function AppearanceSection({
  get,
  set,
}: { get: PrefsGet; set: PrefsSet }): React.JSX.Element {
  const { theme, themes, effectiveAccent, effectiveGold } = useTheme();
  const applyTheme = useApplyTheme();
  const accentColor = usePrefs().accentColor ?? null;
  const commitAccent = useApplyAccent();
  const goldColor = usePrefs().goldColor ?? null;
  const commitGold = useApplyGold();
  const GOLDS = ['#d4a24e', '#fbbf24', '#f59e0b', '#eab308', '#ca8a04', '#b45309'];
  const ACCENTS = [
    '#4fd1c5',
    '#38bdf8',
    '#60a5fa',
    '#818cf8',
    '#a78bfa',
    '#34d399',
    '#fbbf24',
    '#fb7185',
  ];
  return (
    <div className="flex flex-col gap-4">
      <Field label="Theme">
        <select
          value={theme}
          onChange={(e) => applyTheme(e.target.value)}
          className="h-9 w-full rounded-md border border-border bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="auto">Auto (system)</option>
          {themes.map((t) => (
            <option key={t.name} value={t.name}>
              {t.label}
            </option>
          ))}
        </select>
      </Field>
      <Field label="Accent color">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => commitAccent(null)}
            className={cn(
              'rounded-md border px-2 py-1 text-xs',
              accentColor == null
                ? 'border-mneme-cyan ring-1 ring-mneme-cyan'
                : 'border-border hover:bg-muted',
            )}
          >
            Theme default
          </button>
          {ACCENTS.map((c) => (
            <button
              key={c}
              type="button"
              onClick={() => commitAccent(c)}
              aria-label={`Accent ${c}`}
              className={cn(
                'h-6 w-6 rounded-full border-2',
                accentColor === c ? 'border-foreground' : 'border-transparent',
              )}
              style={{ backgroundColor: c }}
            />
          ))}
          <ColorPopover
            value={accentColor ?? effectiveAccent}
            preview={applyAccent}
            commit={commitAccent}
            label="Custom accent color"
          />
        </div>
      </Field>
      <Field label="Secondary accent (gold)">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => commitGold(null)}
            className={cn(
              'rounded-md border px-2 py-1 text-xs',
              goldColor == null
                ? 'border-mneme-cyan ring-1 ring-mneme-cyan'
                : 'border-border hover:bg-muted',
            )}
          >
            Theme default
          </button>
          {GOLDS.map((c) => (
            <button
              key={c}
              type="button"
              onClick={() => commitGold(c)}
              aria-label={`Gold ${c}`}
              className={cn(
                'h-6 w-6 rounded-full border-2',
                goldColor === c ? 'border-foreground' : 'border-transparent',
              )}
              style={{ backgroundColor: c }}
            />
          ))}
          <ColorPopover
            value={goldColor ?? effectiveGold}
            preview={applyGold}
            commit={commitGold}
            label="Custom secondary accent color"
          />
        </div>
      </Field>
      <Field label="File-tree font size (px)">
        <Input
          type="number"
          min={11}
          max={16}
          value={get('treeFontSize') ?? 13}
          onChange={(e) => set('treeFontSize', Number(e.target.value))}
        />
      </Field>
    </div>
  );
}
