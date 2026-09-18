import { getVersion } from '@tauri-apps/api/app';
import type React from 'react';
import { useEffect, useState } from 'react';
import { type ViewMode, selectDefaultViewMode, useEditorStore } from '../../../stores/editor.ts';
import { Button } from '../../ui/button.tsx';
import { Field } from '../controls.tsx';

export function GeneralSection(): React.JSX.Element {
  const defaultViewMode = useEditorStore(selectDefaultViewMode);
  const setDefaultViewMode = useEditorStore((s) => s.setDefaultViewMode);
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);

  return (
    <div className="flex flex-col gap-4">
      <Field label="Version">
        <p className="text-sm text-muted-foreground">
          Mneme {version ?? '—'} <span className="text-xs">(local build)</span>
        </p>
      </Field>
      <Field label="Default view mode (new notes)">
        <div className="flex gap-2">
          {(['editor', 'split', 'preview'] as const satisfies readonly ViewMode[]).map((m) => (
            <Button
              key={m}
              type="button"
              variant={defaultViewMode === m ? 'default' : 'outline'}
              size="sm"
              onClick={() => setDefaultViewMode(m)}
            >
              {m}
            </Button>
          ))}
        </div>
        <p className="text-xs text-muted-foreground">Applies immediately.</p>
      </Field>
    </div>
  );
}
