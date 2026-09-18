import type { Well } from '@mneme/shared';
import { FolderPlus, Pencil, ShieldAlert, Trash2 } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { api } from '../../../lib/api-client.ts';
import { isProtectedLocation } from '../../../lib/macos-permissions.ts';
import {
  useActivateWell,
  useHostHome,
  useRemoveWell,
  useUpdateWell,
  useWells,
} from '../../../lib/wells.ts';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '../../ui/alert-dialog.tsx';
import { Button } from '../../ui/button.tsx';
import { Input } from '../../ui/input.tsx';
import { AddWellDialog } from '../../well/AddWellDialog.tsx';
import { ColorPopover } from './ColorPopover.tsx';

export function WellsSection(): React.JSX.Element {
  const wells = useWells();
  const update = useUpdateWell();
  const remove = useRemoveWell();
  const activate = useActivateWell();
  const home = useHostHome().data?.path ?? '';
  const [addOpen, setAddOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');
  const [confirmRemove, setConfirmRemove] = useState<Well | null>(null);

  const activeId = wells.data?.activeWellId ?? null;

  const onSaveRename = async () => {
    if (!editingId) return;
    await update.mutateAsync({ id: editingId, patch: { name: editingName } });
    setEditingId(null);
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">Folders Mneme reads notes from.</p>
        <Button size="sm" onClick={() => setAddOpen(true)}>
          <FolderPlus className="h-4 w-4" /> Add well
        </Button>
      </div>

      <div className="flex flex-col gap-2">
        {wells.data?.wells.map((v) => {
          const protectedLoc = isProtectedLocation(v.path, home);
          return (
            <div
              key={v.id}
              className="flex flex-col gap-2 rounded-md border border-border bg-background p-3"
            >
              <div className="flex items-center gap-2">
                <span
                  className="h-3 w-3 shrink-0 rounded-full border border-border"
                  style={{ backgroundColor: v.colorTag ?? 'transparent' }}
                  aria-hidden="true"
                />
                {editingId === v.id ? (
                  <div className="flex flex-1 items-center gap-2">
                    <Input
                      value={editingName}
                      onChange={(e) => setEditingName(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') void onSaveRename();
                        if (e.key === 'Escape') setEditingId(null);
                      }}
                      autoFocus
                      className="h-7"
                    />
                    <Button size="sm" onClick={onSaveRename} disabled={update.isPending}>
                      Save
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setEditingId(null)}>
                      Cancel
                    </Button>
                  </div>
                ) : (
                  <span className="flex-1 truncate font-medium">{v.name}</span>
                )}
                {v.id === activeId && (
                  <span className="rounded-sm bg-mneme-cyan/15 px-1.5 py-0.5 text-xs text-mneme-cyan">
                    active
                  </span>
                )}
                {protectedLoc && (
                  <span
                    className="flex items-center gap-1 rounded-sm bg-mneme-warning/15 px-1.5 py-0.5 text-xs text-mneme-warning"
                    title="In a macOS-protected folder (Documents/Desktop/Downloads/iCloud) — may re-prompt for access"
                  >
                    <ShieldAlert className="h-3 w-3" /> macOS-protected
                  </span>
                )}
              </div>
              <div className="truncate font-mono text-xs text-muted-foreground" title={v.path}>
                {v.path}
              </div>
              {editingId !== v.id && (
                <div className="flex flex-wrap items-center gap-2">
                  {v.id !== activeId && (
                    <Button size="sm" variant="outline" onClick={() => activate.mutate(v.id)}>
                      Set active
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setEditingId(v.id);
                      setEditingName(v.name);
                    }}
                  >
                    <Pencil className="h-3.5 w-3.5" /> Rename
                  </Button>
                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    Color
                    <ColorPopover
                      value={v.colorTag ?? '#888888'}
                      commit={(hex) => update.mutate({ id: v.id, patch: { colorTag: hex } })}
                      label={`Color for ${v.name}`}
                    />
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="ml-auto"
                    onClick={() => setConfirmRemove(v)}
                    aria-label={`Remove ${v.name}`}
                  >
                    <Trash2 className="h-3.5 w-3.5 text-mneme-danger" />
                  </Button>
                </div>
              )}
            </div>
          );
        })}
        {wells.data?.wells.length === 0 && (
          <p className="py-6 text-center text-sm italic text-muted-foreground">
            No wells yet. Add one to get started.
          </p>
        )}
      </div>

      <div className="flex flex-col gap-2 rounded-md border border-border bg-muted/30 p-3">
        <p className="text-sm font-medium">Folder access (macOS)</p>
        <p className="text-xs text-muted-foreground">
          Mneme can't grant folder access from here — that's controlled by macOS. If a well keeps
          asking for permission each launch, grant it once in Privacy → Files and Folders, or move
          the folder out of Documents/Desktop and re-add the well.
        </p>
        <div>
          <Button size="sm" variant="outline" onClick={() => void api.system.openPrivacySettings()}>
            Open macOS Privacy Settings
          </Button>
        </div>
      </div>

      <AddWellDialog open={addOpen} onOpenChange={setAddOpen} />

      <AlertDialog open={confirmRemove != null} onOpenChange={(o) => !o && setConfirmRemove(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove well from registry?</AlertDialogTitle>
            <AlertDialogDescription>
              The folder <span className="font-mono text-mneme-cyan">{confirmRemove?.path}</span>{' '}
              and its contents will NOT be deleted — only the registration is removed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={async () => {
                if (confirmRemove) await remove.mutateAsync(confirmRemove.id);
                setConfirmRemove(null);
              }}
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
