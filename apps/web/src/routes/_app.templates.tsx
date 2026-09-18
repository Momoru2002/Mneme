import { useQueryClient } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { Copy, Download, FileText, Plus, Trash2 } from 'lucide-react';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { MarkdownEditor } from '../components/editor/MarkdownEditor.tsx';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '../components/ui/alert-dialog.tsx';
import { Button } from '../components/ui/button.tsx';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog.tsx';
import { Input } from '../components/ui/input.tsx';
import { Label } from '../components/ui/label.tsx';
import {
  TEMPLATES_KEY,
  useCreateTemplate,
  useDeleteTemplate,
  useDuplicateTemplate,
  useImportDefaults,
  useTemplate,
  useTemplates,
  useUpdateTemplate,
} from '../lib/templates.ts';
import { useDocumentTitle } from '../lib/use-document-title.ts';

export const Route = createFileRoute('/_app/templates')({
  component: TemplatesPage,
});

const SAVE_DEBOUNCE_MS = 1500;

function TemplatesPage(): React.JSX.Element {
  useDocumentTitle('Templates');
  const list = useTemplates();
  const [selected, setSelected] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const qc = useQueryClient();

  const importDefaults = useImportDefaults();
  const duplicate = useDuplicateTemplate();
  const remove = useDeleteTemplate();

  useEffect(() => {
    if (!selected && list.data && list.data.templates.length > 0) {
      setSelected(list.data.templates[0]?.name ?? null);
    }
  }, [list.data, selected]);

  return (
    <div className="flex h-full overflow-hidden">
      <aside className="flex w-64 shrink-0 flex-col border-r border-border bg-card">
        <div className="flex items-center justify-between px-3 py-2">
          <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Templates
          </span>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setCreateOpen(true)}
              aria-label="New"
            >
              <Plus className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => importDefaults.mutate()}
              disabled={importDefaults.isPending}
              aria-label="Import defaults"
            >
              <Download className="h-4 w-4" />
            </Button>
          </div>
        </div>
        <div className="flex-1 overflow-y-auto">
          {list.isLoading && (
            <div className="px-3 py-4 text-xs text-muted-foreground">Loading…</div>
          )}
          {list.data && list.data.templates.length === 0 && (
            <div className="px-4 py-6 text-center text-xs italic text-muted-foreground">
              No templates yet. Click + to create one, or the download icon to import Coursera
              defaults.
            </div>
          )}
          {list.data?.templates.map((t) => (
            // Row is a flex container, not a button — a button must not nest
            // buttons (invalid HTML / WCAG 4.1.2). Select + duplicate + delete
            // are sibling buttons; the action buttons stay keyboard-reachable.
            <div
              key={t.name}
              className={`group flex w-full items-center gap-2 px-3 text-sm hover:bg-muted ${
                selected === t.name ? 'bg-muted text-foreground' : ''
              }`}
            >
              <button
                type="button"
                onClick={() => setSelected(t.name)}
                className="flex min-w-0 flex-1 items-center gap-2 py-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-mneme-cyan"
              >
                <FileText
                  className="h-3.5 w-3.5 shrink-0 text-muted-foreground"
                  aria-hidden="true"
                />
                <span className="min-w-0 flex-1 truncate">{t.name}</span>
              </button>
              <button
                type="button"
                onClick={() => duplicate.mutate(t.name)}
                className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 hover:text-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mneme-cyan group-hover:opacity-100"
                aria-label={`Duplicate ${t.name}`}
              >
                <Copy className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                onClick={() => setConfirmDelete(t.name)}
                className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 hover:text-mneme-danger focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mneme-cyan group-hover:opacity-100"
                aria-label={`Delete ${t.name}`}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
          ))}
        </div>
      </aside>

      <div className="flex-1 overflow-hidden">
        {selected ? (
          <TemplateEditor name={selected} />
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-muted-foreground italic">
            Select a template to edit or click + to create one.
          </div>
        )}
      </div>

      <CreateTemplateDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onCreated={(name) => {
          setSelected(name);
          qc.invalidateQueries({ queryKey: TEMPLATES_KEY });
        }}
      />

      <AlertDialog open={confirmDelete != null} onOpenChange={(o) => !o && setConfirmDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete template?</AlertDialogTitle>
            <AlertDialogDescription>
              <span className="font-mono text-mneme-cyan">{confirmDelete}</span> will be removed
              permanently.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={async () => {
                if (confirmDelete) {
                  await remove.mutateAsync(confirmDelete);
                  if (selected === confirmDelete) setSelected(null);
                }
                setConfirmDelete(null);
              }}
              className="bg-destructive text-destructive-foreground hover:brightness-110"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

function TemplateEditor({ name }: { name: string }): React.JSX.Element {
  const tmpl = useTemplate(name);
  const update = useUpdateTemplate();
  const [buffer, setBuffer] = useState('');
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (tmpl.data) setBuffer(tmpl.data.content);
  }, [tmpl.data]);

  const onChange = (next: string) => {
    setBuffer(next);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      await update.mutateAsync({ name, input: { content: next } });
      setSavedAt(Date.now());
    }, SAVE_DEBOUNCE_MS);
  };

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-3 border-b border-border bg-card px-4 py-2">
        <FileText className="h-4 w-4 text-muted-foreground" />
        <h2 className="font-mono text-sm truncate">{name}</h2>
        <span className="ml-auto text-xs text-muted-foreground">
          {update.isPending
            ? 'Saving…'
            : savedAt
              ? `Saved ${new Date(savedAt).toLocaleTimeString()}`
              : ''}
        </span>
      </header>
      <div className="flex-1">
        <MarkdownEditor value={buffer} onChange={onChange} />
      </div>
    </div>
  );
}

function CreateTemplateDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: (name: string) => void;
}): React.JSX.Element {
  const [name, setName] = useState('');
  const create = useCreateTemplate();

  useEffect(() => {
    if (open) setName('');
  }, [open]);

  const submit = async () => {
    if (!name.trim()) return;
    const tmpl = await create.mutateAsync({ name: name.trim(), content: '' });
    onCreated(tmpl.name);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New template</DialogTitle>
          <DialogDescription>
            Templates are reusable note skeletons. Variables like <code>{'{{title}}'}</code> or{' '}
            <code>{'{{date}}'}</code> get filled when you apply.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-2">
          <Label htmlFor="template-name">Name</Label>
          <Input
            id="template-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Daily journal"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submit();
            }}
          />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!name.trim() || create.isPending}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
