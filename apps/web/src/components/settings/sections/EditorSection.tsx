import type React from 'react';
import { Button } from '../../ui/button.tsx';
import { Input } from '../../ui/input.tsx';
import { Field, type PrefsGet, type PrefsSet, Toggle } from '../controls.tsx';

export function EditorSection({ get, set }: { get: PrefsGet; set: PrefsSet }): React.JSX.Element {
  return (
    <div className="flex flex-col gap-4">
      <Field label="Auto-save interval (ms)">
        <Input
          type="number"
          min={500}
          max={60000}
          step={500}
          value={get('autoSaveMs') ?? 2000}
          onChange={(e) => set('autoSaveMs', Number(e.target.value))}
        />
      </Field>
      <Field label="Font size (px)">
        <Input
          type="number"
          min={10}
          max={24}
          value={get('fontSize') ?? 14}
          onChange={(e) => set('fontSize', Number(e.target.value))}
        />
      </Field>
      <Field label="Tab width">
        <Input
          type="number"
          min={2}
          max={8}
          value={get('tabWidth') ?? 2}
          onChange={(e) => set('tabWidth', Number(e.target.value))}
        />
      </Field>
      <Field label="Indent style">
        <div className="flex gap-2">
          {(['spaces', 'tabs'] as const).map((s) => (
            <Button
              key={s}
              type="button"
              variant={(get('indentStyle') ?? 'spaces') === s ? 'default' : 'outline'}
              size="sm"
              onClick={() => set('indentStyle', s)}
            >
              {s}
            </Button>
          ))}
        </div>
      </Field>
      <Toggle
        label="Line numbers"
        value={Boolean(get('lineNumbers') ?? true)}
        onChange={(v) => set('lineNumbers', v)}
      />
      <Toggle
        label="Word wrap"
        value={Boolean(get('wordWrap') ?? true)}
        onChange={(v) => set('wordWrap', v)}
      />
      <Toggle
        label="Mermaid diagrams"
        value={Boolean(get('previewMermaid') ?? true)}
        onChange={(v) => set('previewMermaid', v)}
      />
    </div>
  );
}
