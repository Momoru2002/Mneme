import { zodResolver } from '@hookform/resolvers/zod';
import { type AddWellInput, addWellInputSchema } from '@mneme/shared';
import { useMutation } from '@tanstack/react-query';
import { CheckCircle2, XCircle } from 'lucide-react';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { api } from '../../lib/api-client.ts';
import { useAddWell } from '../../lib/wells.ts';
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs.tsx';
import { HostBrowser } from './HostBrowser.tsx';

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

function basename(p: string): string {
  const trimmed = p.replace(/\/+$/, '');
  const i = trimmed.lastIndexOf('/');
  return i === -1 ? trimmed : trimmed.slice(i + 1);
}

export function AddWellDialog({ open, onOpenChange }: Props): React.JSX.Element {
  const [tab, setTab] = useState<'browse' | 'paste'>('browse');
  const [browsePath, setBrowsePath] = useState<string | null>(null);
  // Whether the user has typed their own Name (stop auto-deriving it from path).
  const [nameTouched, setNameTouched] = useState(false);
  const addWell = useAddWell();

  const form = useForm<AddWellInput>({
    resolver: zodResolver(addWellInputSchema),
    defaultValues: { name: '', path: '' },
  });

  const path = form.watch('path');

  const validate = useMutation({
    mutationFn: (p: string) => api.wells.validate(p),
  });

  // When switching tabs or picking from browser, fill the form's path.
  // form is a stable ref from react-hook-form — exclude from deps to avoid loop.
  // biome-ignore lint/correctness/useExhaustiveDependencies: form is stable
  useEffect(() => {
    if (browsePath) form.setValue('path', browsePath, { shouldValidate: true });
  }, [browsePath]);

  // Auto-derive the Name from the path's basename until the user edits it, so a
  // path is enough to add a well without manually naming it.
  // biome-ignore lint/correctness/useExhaustiveDependencies: form is stable
  useEffect(() => {
    if (!nameTouched && path) form.setValue('name', basename(path));
  }, [path, nameTouched]);

  // The host browser reports the folder currently being viewed; treat it as the
  // default path candidate so "Add well" works without an explicit Choose.
  const onBrowseLocation = useCallback((p: string) => setBrowsePath(p), []);

  // useMutation returns a NEW object every render — including it in deps causes
  // an infinite loop. The closure captures the latest reset() through the
  // mutation observer subscription internals; safe to exclude.
  // biome-ignore lint/correctness/useExhaustiveDependencies: form and validate are stable / safe to omit
  useEffect(() => {
    if (!open) {
      form.reset();
      setBrowsePath(null);
      setNameTouched(false);
      setTab('browse');
      validate.reset();
    }
  }, [open]);

  const onSubmit = form.handleSubmit(async (values) => {
    await addWell.mutateAsync(values);
    onOpenChange(false);
  });

  const onValidateClick = async () => {
    if (!path) return;
    await validate.mutateAsync(path);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Add Well</DialogTitle>
          <DialogDescription>
            Register an Obsidian-style folder of markdown notes. Files stay on disk; Mneme only
            tracks the path.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <Tabs value={tab} onValueChange={(v) => setTab(v as 'browse' | 'paste')}>
            <TabsList>
              <TabsTrigger value="browse">Browse Host</TabsTrigger>
              <TabsTrigger value="paste">Paste Path</TabsTrigger>
            </TabsList>
            <TabsContent value="browse">
              <HostBrowser
                onSelect={(p) => {
                  setBrowsePath(p);
                  form.setValue('path', p, { shouldValidate: true });
                }}
                onLocationChange={onBrowseLocation}
                selectedPath={browsePath}
              />
            </TabsContent>
            <TabsContent value="paste">
              <div className="flex flex-col gap-2">
                <Label htmlFor="well-path">Absolute path</Label>
                <div className="flex gap-2">
                  <Input
                    id="well-path"
                    {...form.register('path')}
                    placeholder="/Users/you/Documents/MyWell"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={onValidateClick}
                    disabled={!path || validate.isPending}
                  >
                    {validate.isPending ? 'Checking…' : 'Validate'}
                  </Button>
                </div>
                {form.formState.errors.path && (
                  <span className="text-xs text-destructive">
                    {form.formState.errors.path.message}
                  </span>
                )}
                {validate.data && (
                  // biome-ignore lint/a11y/useSemanticElements: intentional ARIA live region for path-validation results; <output> can't contain the block-level Check rows.
                  <div
                    role="status"
                    aria-live="polite"
                    className="text-xs space-y-1 rounded-md border border-border bg-background p-3"
                  >
                    <Check ok={validate.data.exists} label="Path exists" />
                    <Check ok={validate.data.isDirectory} label="Is a directory" />
                    <Check ok={validate.data.readable} label="Read/write access" />
                    <Check
                      ok={validate.data.isObsidianWell}
                      label="Has .obsidian folder"
                      optional
                    />
                    <p className="text-muted-foreground pt-1">
                      {validate.data.fileCount} files at the top level
                    </p>
                    {validate.data.error && (
                      <p className="text-destructive">Error: {validate.data.error}</p>
                    )}
                  </div>
                )}
              </div>
            </TabsContent>
          </Tabs>

          <div className="flex flex-col gap-2">
            <Label htmlFor="well-name">Name</Label>
            <Input
              id="well-name"
              {...form.register('name')}
              onChange={(e) => {
                setNameTouched(true);
                form.setValue('name', e.target.value, { shouldValidate: true });
              }}
              placeholder="My Study Well"
            />
            {form.formState.errors.name && (
              <span className="text-xs text-destructive">{form.formState.errors.name.message}</span>
            )}
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={addWell.isPending}>
              {addWell.isPending ? 'Adding…' : 'Add well'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Check({
  ok,
  label,
  optional,
}: {
  ok: boolean;
  label: string;
  optional?: boolean;
}): React.JSX.Element {
  const Icon = ok ? CheckCircle2 : XCircle;
  const colorClass = ok
    ? 'text-mneme-success'
    : optional
      ? 'text-muted-foreground'
      : 'text-mneme-danger';
  return (
    <p className={`flex items-center gap-2 ${colorClass}`}>
      <Icon className="h-3.5 w-3.5" /> {label}
      {optional && !ok && <span className="text-muted-foreground italic">(optional)</span>}
    </p>
  );
}
