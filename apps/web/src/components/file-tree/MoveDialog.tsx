import { ChevronDown, ChevronRight, Folder } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useMoveFile } from '../../lib/files.ts';
import { useMoveFolder } from '../../lib/folders.ts';
import { useTree } from '../../lib/tree.ts';
import { Button } from '../ui/button.tsx';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog.tsx';

/** The node being moved (set by a "Move to…" menu item in TreeNode). */
export type MoveTarget = { path: string; name: string; type: 'file' | 'folder' };

function basename(p: string): string {
  const i = p.lastIndexOf('/');
  return i === -1 ? p : p.slice(i + 1);
}
function parentDir(p: string): string {
  const i = p.lastIndexOf('/');
  return i === -1 ? '' : p.slice(0, i);
}
function joinPath(dir: string, name: string): string {
  return dir ? `${dir}/${name}` : name;
}

/** Is `dir` an invalid destination for moving a folder at `sourceFolder`? */
function isInsideSelf(dir: string, sourceFolder: string | null): boolean {
  if (sourceFolder == null) return false;
  return dir === sourceFolder || dir.startsWith(`${sourceFolder}/`);
}

type RowProps = {
  wellId: string;
  path: string;
  name: string;
  depth: number;
  selected: string | null;
  onSelect: (dir: string) => void;
  disabledPrefix: string | null;
};

/** One expandable folder row in the destination picker (lazy, recursive). */
function FolderRow({
  wellId,
  path,
  name,
  depth,
  selected,
  onSelect,
  disabledPrefix,
}: RowProps): React.JSX.Element {
  const [expanded, setExpanded] = useState(false);
  // Only fetch children once expanded.
  const tree = useTree(wellId, expanded ? path : '');
  const childFolders = expanded
    ? (tree.data?.entries ?? []).filter((e) => e.type === 'folder')
    : [];
  const isSelected = selected === path;
  const disabled = isInsideSelf(path, disabledPrefix);

  return (
    <li role="treeitem" aria-expanded={expanded} aria-selected={isSelected}>
      <div className="flex items-center" style={{ paddingLeft: `${depth * 14}px` }}>
        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          aria-label={expanded ? `Collapse ${name}` : `Expand ${name}`}
          className="flex h-6 w-6 shrink-0 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mneme-cyan"
        >
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5" />
          )}
        </button>
        <button
          type="button"
          disabled={disabled}
          onClick={() => onSelect(path)}
          aria-pressed={isSelected}
          className={`flex min-w-0 flex-1 items-center gap-1.5 rounded-sm px-1.5 py-1 text-left text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mneme-cyan disabled:opacity-40 ${
            isSelected ? 'bg-mneme-cyan/15 text-mneme-cyan' : 'hover:bg-muted'
          }`}
          title={disabled ? "Can't move a folder into itself" : path}
        >
          <Folder className="h-3.5 w-3.5 shrink-0 text-mneme-gold" aria-hidden="true" />
          <span className="min-w-0 truncate">{name}</span>
        </button>
      </div>
      {expanded && childFolders.length > 0 && (
        <ul>
          {childFolders.map((f) => (
            <FolderRow
              key={f.path}
              wellId={wellId}
              path={f.path}
              name={f.name}
              depth={depth + 1}
              selected={selected}
              onSelect={onSelect}
              disabledPrefix={disabledPrefix}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

type Props = {
  wellId: string;
  target: MoveTarget | null;
  onOpenChange: (open: boolean) => void;
};

/**
 * Keyboard-accessible "Move to…" destination picker (WCAG 2.1.1 + 2.5.7 — the
 * non-drag alternative to the file-tree drag-and-drop). Browses the well's
 * folders lazily via the existing `tree` command and calls the same
 * useMoveFile / useMoveFolder mutations the drag handler uses.
 */
export function MoveDialog({ wellId, target, onOpenChange }: Props): React.JSX.Element {
  const [dest, setDest] = useState<string | null>(null);
  const moveFile = useMoveFile(wellId);
  const moveFolder = useMoveFolder(wellId);

  const open = target != null;
  const rootTree = useTree(wellId, open ? '' : '');
  const rootFolders = open ? (rootTree.data?.entries ?? []).filter((e) => e.type === 'folder') : [];

  const sourceParent = target ? parentDir(target.path) : '';
  const disabledPrefix = target?.type === 'folder' ? target.path : null;
  const canMove =
    target != null &&
    dest != null &&
    dest !== sourceParent && // no-op into current parent
    !isInsideSelf(dest, disabledPrefix);

  const close = () => {
    onOpenChange(false);
    setDest(null);
  };

  const submit = async () => {
    if (!target || dest == null) return;
    const destPath = joinPath(dest, basename(target.path));
    const input = { sourcePath: target.path, destPath };
    if (target.type === 'folder') await moveFolder.mutateAsync(input);
    else await moveFile.mutateAsync(input);
    close();
  };

  const pending = moveFile.isPending || moveFolder.isPending;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Move "{target?.name}"</DialogTitle>
          <DialogDescription>
            Choose a destination folder. Use Tab and the arrow buttons to browse, then Move.
          </DialogDescription>
        </DialogHeader>

        <ul
          role="tree"
          aria-label="Destination folders"
          className="max-h-72 overflow-auto rounded-md border border-border bg-background p-1"
        >
          <li role="treeitem" aria-selected={dest === ''}>
            <button
              type="button"
              onClick={() => setDest('')}
              aria-pressed={dest === ''}
              className={`flex w-full items-center gap-1.5 rounded-sm px-1.5 py-1 text-left text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mneme-cyan ${
                dest === '' ? 'bg-mneme-cyan/15 text-mneme-cyan' : 'hover:bg-muted'
              }`}
            >
              <Folder className="h-3.5 w-3.5 shrink-0 text-mneme-gold" aria-hidden="true" /> Well
              root
            </button>
          </li>
          {rootFolders.map((f) => (
            <FolderRow
              key={f.path}
              wellId={wellId}
              path={f.path}
              name={f.name}
              depth={1}
              selected={dest}
              onSelect={setDest}
              disabledPrefix={disabledPrefix}
            />
          ))}
        </ul>

        <DialogFooter>
          <Button variant="outline" onClick={close}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!canMove || pending}>
            {pending ? 'Moving…' : 'Move'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
