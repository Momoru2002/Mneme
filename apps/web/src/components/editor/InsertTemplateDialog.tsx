import { Layers } from 'lucide-react';
import type React from 'react';
import { useEffect, useMemo, useState } from 'react';
import { useApplyTemplate, useTemplate, useTemplates } from '../../lib/templates.ts';
import { Button } from '../ui/button.tsx';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog.tsx';
import { Input } from '../ui/input.tsx';
import { Label } from '../ui/label.tsx';

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onInsert: (rendered: string) => void;
};

function extractVars(content: string): string[] {
  const found = new Set<string>();
  for (const m of content.matchAll(/\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}/g)) {
    const name = m[1];
    if (!name) continue;
    if (name === 'cursor' || name === 'date' || name === 'datetime') continue;
    found.add(name);
  }
  return [...found].sort();
}

export function InsertTemplateDialog({ open, onOpenChange, onInsert }: Props): React.JSX.Element {
  const list = useTemplates();
  const [selected, setSelected] = useState<string | null>(null);
  const tmpl = useTemplate(open ? selected : null);
  const apply = useApplyTemplate();

  const vars = useMemo(() => (tmpl.data ? extractVars(tmpl.data.content) : []), [tmpl.data]);
  const [values, setValues] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!open) {
      setSelected(null);
      setValues({});
    }
  }, [open]);

  useEffect(() => {
    setValues((prev) => {
      const next: Record<string, string> = {};
      for (const v of vars) next[v] = prev[v] ?? '';
      return next;
    });
  }, [vars]);

  const onSubmit = async () => {
    if (!selected) return;
    const result = await apply.mutateAsync({ name: selected, input: { vars: values } });
    onInsert(result.rendered);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Insert template</DialogTitle>
          <DialogDescription>
            Pick a template, fill any variables, and insert the rendered content at the cursor.
          </DialogDescription>
        </DialogHeader>

        <div className="grid grid-cols-[200px_1fr] gap-4">
          <aside className="rounded-md border border-border bg-background max-h-[50vh] overflow-y-auto">
            {list.data?.templates.map((t) => (
              <button
                key={t.name}
                type="button"
                onClick={() => setSelected(t.name)}
                className={`flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-muted ${
                  selected === t.name ? 'bg-muted text-foreground' : ''
                }`}
              >
                <Layers className="h-3.5 w-3.5 text-mneme-gold" />
                <span className="truncate">{t.name}</span>
              </button>
            ))}
            {(list.data?.templates.length ?? 0) === 0 && (
              <p className="p-4 text-xs italic text-muted-foreground">
                No templates available. Visit the Templates page to create some.
              </p>
            )}
          </aside>

          <div className="flex flex-col gap-3">
            {!selected && (
              <p className="text-sm italic text-muted-foreground">Select a template to continue.</p>
            )}
            {selected && vars.length === 0 && (
              <p className="text-sm text-muted-foreground">
                This template uses only built-in variables ({'{{date}}'}, {'{{datetime}}'}). Click
                Insert.
              </p>
            )}
            {selected &&
              vars.map((v) => (
                <div key={v} className="flex flex-col gap-1">
                  <Label htmlFor={`var-${v}`}>{`{{${v}}}`}</Label>
                  <Input
                    id={`var-${v}`}
                    value={values[v] ?? ''}
                    onChange={(e) => setValues((p) => ({ ...p, [v]: e.target.value }))}
                    placeholder={v === 'title' ? 'Note title' : v}
                  />
                </div>
              ))}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={onSubmit} disabled={!selected || apply.isPending}>
            {apply.isPending ? 'Inserting…' : 'Insert'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
