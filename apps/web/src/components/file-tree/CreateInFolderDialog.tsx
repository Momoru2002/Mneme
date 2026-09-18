import path from 'path-browserify';
import type React from 'react';
import { useEffect, useState } from 'react';
import { useCreateFile } from '../../lib/files.ts';
import { useCreateFolder } from '../../lib/folders.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
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
  wellId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  folderPath: string; // '' means well root
  kind: 'file' | 'folder';
};

export function CreateInFolderDialog({
  wellId,
  open,
  onOpenChange,
  folderPath,
  kind,
}: Props): React.JSX.Element {
  const [name, setName] = useState('');
  const openInActiveGroup = useWorkspaceStore((s) => s.openInActiveGroup);
  const createFile = useCreateFile(wellId);
  const createFolder = useCreateFolder(wellId);

  useEffect(() => {
    if (open) setName('');
  }, [open]);

  const submit = async () => {
    const trimmed = name.trim();
    if (!trimmed || !wellId) return;
    const finalName = kind === 'file' && !trimmed.endsWith('.md') ? `${trimmed}.md` : trimmed;
    const finalPath = folderPath ? path.posix.join(folderPath, finalName) : finalName;
    if (kind === 'file') {
      const result = await createFile.mutateAsync({ path: finalPath, content: '' });
      openInActiveGroup({ wellId, path: result.path });
    } else {
      await createFolder.mutateAsync({ path: finalPath });
    }
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New {kind}</DialogTitle>
          <DialogDescription>
            In: <span className="font-mono text-mneme-cyan">{folderPath || '(well root)'}</span>
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-2">
          <Label htmlFor="entry-name">Name</Label>
          <Input
            id="entry-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submit();
            }}
            placeholder={kind === 'file' ? 'my-note' : 'My Folder'}
            autoFocus
          />
          {kind === 'file' && (
            <p className="text-xs text-muted-foreground">.md will be appended if omitted.</p>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={submit}
            disabled={!name.trim() || createFile.isPending || createFolder.isPending}
          >
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
