import {
  ChevronDown,
  ChevronRight,
  Copy,
  FileText,
  Folder,
  FolderInput,
  FolderOpen,
  FolderPlus,
  Pencil,
  Plus,
  Trash2,
} from 'lucide-react';
import path from 'path-browserify';
import type React from 'react';
import { useEffect, useState } from 'react';
import { cn } from '../../lib/cn.ts';
import { useDeleteFile, useDuplicateFile, useMoveFile, useRenameFile } from '../../lib/files.ts';
import { useDeleteFolder, useMoveFolder, useRenameFolder } from '../../lib/folders.ts';
import { useTree } from '../../lib/tree.ts';
import { insideOrEq, useFileTreeStore } from '../../stores/file-tree.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '../ui/alert-dialog.tsx';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from '../ui/context-menu.tsx';
import { Input } from '../ui/input.tsx';
import type { MoveTarget } from './MoveDialog.tsx';

type Props = {
  wellId: string;
  name: string;
  path: string;
  type: 'folder' | 'file';
  hasChildren: boolean;
  /** M14: when true, write affordances (rename/delete/move/create) are hidden. */
  readOnly?: boolean;
  onCreateInFolder?: (folderPath: string, kind: 'file' | 'folder') => void;
  onMove?: (target: MoveTarget) => void;
};

export function TreeNode(props: Props): React.JSX.Element {
  const {
    wellId,
    name,
    path: nodePath,
    type,
    hasChildren,
    readOnly = false,
    onCreateInFolder,
    onMove,
  } = props;
  const openInActiveGroup = useWorkspaceStore((s) => s.openInActiveGroup);
  const renamePathInTabs = useWorkspaceStore((s) => s.renamePathInTabs);
  const removePathFromTabs = useWorkspaceStore((s) => s.removePathFromTabs);
  // Highlight ONLY the single active file — the focused pane's active tab — not
  // every open tab; otherwise a 3-pane split would light up 3 rows in the file
  // panel. selectActivePath returns a stable reference (the stored tab object, or
  // null), so subscribing to it directly is safe (no useShallow / no React #185).
  const activePath = useWorkspaceStore((s) => s.selectActivePath());

  const expanded = useFileTreeStore((s) => (s.expandedByWell[wellId] ?? []).includes(nodePath));
  const toggleExpanded = useFileTreeStore((s) => s.toggleExpanded);
  const setHoverPath = useFileTreeStore((s) => s.setHoverPath);

  // If this row was the hovered one and it unmounts (e.g. a background refetch
  // removes it while the cursor sits still), drop the now-stale highlight target
  // so an ancestor guide doesn't stay lit. Reads live state to avoid re-subscribing.
  useEffect(() => {
    return () => {
      if (useFileTreeStore.getState().hoverPath === nodePath) {
        useFileTreeStore.getState().setHoverPath(null);
      }
    };
  }, [nodePath]);

  const [renaming, setRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(name);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const renameFile = useRenameFile(wellId);
  const renameFolder = useRenameFolder(wellId);
  const deleteFile = useDeleteFile(wellId);
  const deleteFolder = useDeleteFolder(wellId);
  const duplicateFile = useDuplicateFile(wellId);
  const moveFile = useMoveFile(wellId);
  const moveFolder = useMoveFolder(wellId);

  const isActive = type === 'file' && activePath?.wellId === wellId && activePath.path === nodePath;
  const dir = nodePath.includes('/') ? nodePath.slice(0, nodePath.lastIndexOf('/')) : '';

  const onCommitRename = async () => {
    const trimmed = renameValue.trim();
    if (!trimmed || trimmed === name) {
      setRenaming(false);
      setRenameValue(name);
      return;
    }
    const newPath = dir ? path.posix.join(dir, trimmed) : trimmed;
    if (type === 'file') {
      const finalNew = newPath.endsWith('.md') ? newPath : `${newPath}.md`;
      await renameFile.mutateAsync({ oldPath: nodePath, newPath: finalNew });
      renamePathInTabs(wellId, nodePath, finalNew);
    } else {
      await renameFolder.mutateAsync({ oldPath: nodePath, newPath });
    }
    setRenaming(false);
  };

  const onDelete = async () => {
    if (type === 'file') {
      await deleteFile.mutateAsync(nodePath);
      removePathFromTabs(wellId, nodePath);
    } else {
      await deleteFolder.mutateAsync(nodePath);
    }
    setConfirmDelete(false);
  };

  const onDuplicate = async () => {
    if (type !== 'file') return;
    await duplicateFile.mutateAsync({ path: nodePath });
  };

  // Drag-and-drop
  const onDragStart = (e: React.DragEvent) => {
    e.dataTransfer.setData('application/x-mneme-path', nodePath);
    e.dataTransfer.setData('application/x-mneme-type', type);
    e.dataTransfer.effectAllowed = 'move';
  };

  const onDragOver = (e: React.DragEvent) => {
    if (type !== 'folder') return;
    const sourceType = e.dataTransfer.types.includes('application/x-mneme-type');
    if (!sourceType) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  const onDrop = async (e: React.DragEvent) => {
    if (type !== 'folder') return;
    e.preventDefault();
    const sourcePath = e.dataTransfer.getData('application/x-mneme-path');
    const sourceType = e.dataTransfer.getData('application/x-mneme-type') as 'file' | 'folder';
    if (!sourcePath || sourcePath === nodePath) return;
    if (
      sourceType === 'folder' &&
      (nodePath === sourcePath || nodePath.startsWith(`${sourcePath}/`))
    )
      return;
    const basename = sourcePath.split('/').pop() ?? sourcePath;
    const destPath = path.posix.join(nodePath, basename);
    if (sourceType === 'file') {
      await moveFile.mutateAsync({ sourcePath, destPath });
      renamePathInTabs(wellId, sourcePath, destPath);
    } else {
      await moveFolder.mutateAsync({ sourcePath, destPath });
    }
  };

  if (type === 'file') {
    return (
      <>
        <ContextMenu>
          <ContextMenuTrigger asChild>
            <div
              className={cn(
                'group flex w-full min-w-0 items-center gap-1 rounded-sm px-2 py-1 hover:bg-muted',
                isActive && 'bg-muted text-foreground',
              )}
              draggable={!renaming && !readOnly}
              onDragStart={readOnly ? undefined : onDragStart}
              onMouseEnter={() => setHoverPath(nodePath)}
            >
              <span className="w-3.5 shrink-0" aria-hidden="true" />
              <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
              {renaming ? (
                <Input
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onBlur={onCommitRename}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onCommitRename();
                    if (e.key === 'Escape') {
                      setRenaming(false);
                      setRenameValue(name);
                    }
                  }}
                  autoFocus
                  className="h-6"
                  style={{ fontSize: 'inherit' }}
                />
              ) : (
                <button
                  type="button"
                  onClick={() => openInActiveGroup({ wellId, path: nodePath })}
                  className="min-w-0 flex-1 truncate text-left"
                  title={name.replace(/\.md$/, '')}
                >
                  {name.replace(/\.md$/, '')}
                </button>
              )}
            </div>
          </ContextMenuTrigger>
          {/* M14: for read-only wells, context menu only shows open (no write ops) */}
          <ContextMenuContent>
            {readOnly ? (
              <ContextMenuItem onSelect={() => openInActiveGroup({ wellId, path: nodePath })}>
                <FileText className="h-3.5 w-3.5" /> Open
              </ContextMenuItem>
            ) : (
              <>
                <ContextMenuItem onSelect={() => setRenaming(true)}>
                  <Pencil className="h-3.5 w-3.5" /> Rename
                </ContextMenuItem>
                <ContextMenuItem onSelect={onDuplicate}>
                  <Copy className="h-3.5 w-3.5" /> Duplicate
                </ContextMenuItem>
                <ContextMenuItem onSelect={() => onMove?.({ path: nodePath, name, type: 'file' })}>
                  <FolderInput className="h-3.5 w-3.5" /> Move to…
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem
                  onSelect={() => setConfirmDelete(true)}
                  className="text-destructive"
                >
                  <Trash2 className="h-3.5 w-3.5" /> Delete
                </ContextMenuItem>
              </>
            )}
          </ContextMenuContent>
        </ContextMenu>

        <DeleteConfirm
          open={confirmDelete}
          onOpenChange={setConfirmDelete}
          onConfirm={onDelete}
          name={name}
          kind="file"
        />
      </>
    );
  }

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div
            onDragOver={readOnly ? undefined : onDragOver}
            onDrop={readOnly ? undefined : onDrop}
          >
            <div
              className={cn(
                'group flex w-full min-w-0 items-center gap-1 rounded-sm px-2 py-1 hover:bg-muted',
                !hasChildren && 'opacity-70',
              )}
              draggable={!renaming && !readOnly}
              onDragStart={readOnly ? undefined : onDragStart}
              onMouseEnter={() => setHoverPath(nodePath)}
            >
              <button
                type="button"
                onClick={() => hasChildren && toggleExpanded(wellId, nodePath)}
                disabled={!hasChildren}
                className="flex items-center"
                aria-label={expanded ? 'Collapse' : 'Expand'}
              >
                {hasChildren ? (
                  expanded ? (
                    <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                  ) : (
                    <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                  )
                ) : (
                  <span className="w-3.5" />
                )}
              </button>
              {expanded ? (
                <FolderOpen className="h-3.5 w-3.5 shrink-0 text-mneme-gold" />
              ) : (
                <Folder className="h-3.5 w-3.5 shrink-0 text-mneme-gold" />
              )}
              {renaming ? (
                <Input
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onBlur={onCommitRename}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onCommitRename();
                    if (e.key === 'Escape') {
                      setRenaming(false);
                      setRenameValue(name);
                    }
                  }}
                  autoFocus
                  className="h-6"
                  style={{ fontSize: 'inherit' }}
                />
              ) : (
                <span
                  className="min-w-0 flex-1 truncate"
                  onDoubleClick={() => hasChildren && toggleExpanded(wellId, nodePath)}
                  title={name}
                >
                  {name}
                </span>
              )}
            </div>

            {expanded && hasChildren && (
              <FolderChildren
                wellId={wellId}
                path={nodePath}
                readOnly={readOnly}
                onCreateInFolder={readOnly ? undefined : onCreateInFolder}
                onMove={readOnly ? undefined : onMove}
              />
            )}
          </div>
        </ContextMenuTrigger>
        {/* M14: for read-only wells, folder context menu has no write ops */}
        <ContextMenuContent>
          {readOnly ? (
            <ContextMenuItem onSelect={() => toggleExpanded(wellId, nodePath)}>
              {expanded ? (
                <ChevronDown className="h-3.5 w-3.5" />
              ) : (
                <ChevronRight className="h-3.5 w-3.5" />
              )}{' '}
              {expanded ? 'Collapse' : 'Expand'}
            </ContextMenuItem>
          ) : (
            <>
              <ContextMenuItem onSelect={() => onCreateInFolder?.(nodePath, 'file')}>
                <Plus className="h-3.5 w-3.5" /> New File
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => onCreateInFolder?.(nodePath, 'folder')}>
                <FolderPlus className="h-3.5 w-3.5" /> New Folder
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onSelect={() => setRenaming(true)}>
                <Pencil className="h-3.5 w-3.5" /> Rename
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => onMove?.({ path: nodePath, name, type: 'folder' })}>
                <FolderInput className="h-3.5 w-3.5" /> Move to…
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onSelect={() => setConfirmDelete(true)} className="text-destructive">
                <Trash2 className="h-3.5 w-3.5" /> Delete (recursive)
              </ContextMenuItem>
            </>
          )}
        </ContextMenuContent>
      </ContextMenu>

      <DeleteConfirm
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        onConfirm={onDelete}
        name={name}
        kind="folder"
      />
    </>
  );
}

function FolderChildren({
  wellId,
  path,
  readOnly,
  onCreateInFolder,
  onMove,
}: {
  wellId: string;
  path: string;
  readOnly?: boolean;
  onCreateInFolder?: (folderPath: string, kind: 'file' | 'folder') => void;
  onMove?: (target: MoveTarget) => void;
}): React.JSX.Element {
  const tree = useTree(wellId, path);
  const hot = useFileTreeStore((s) => insideOrEq(path, s.hoverPath));
  const activePath = useWorkspaceStore((s) => s.selectActivePath());
  const open = activePath?.wellId === wellId && insideOrEq(path, activePath.path);
  const highlighted = hot || open;

  let content: React.ReactNode;
  if (tree.isLoading) {
    content = <div className="px-2 py-1 text-xs text-muted-foreground">Loading…</div>;
  } else if (tree.error) {
    content = (
      <div className="px-2 py-1 text-xs text-destructive">{(tree.error as Error).message}</div>
    );
  } else if (!tree.data || tree.data.entries.length === 0) {
    content = <div className="px-2 py-1 text-xs italic text-muted-foreground">Empty</div>;
  } else {
    content = tree.data.entries.map((entry) => (
      <TreeNode
        key={entry.path}
        wellId={wellId}
        name={entry.name}
        path={entry.path}
        type={entry.type}
        hasChildren={entry.hasChildren}
        readOnly={readOnly}
        onCreateInFolder={readOnly ? undefined : onCreateInFolder}
        onMove={readOnly ? undefined : onMove}
      />
    ));
  }

  return (
    <div
      className={cn('border-l', highlighted ? 'border-mneme-cyan/60' : 'border-border')}
      style={{ marginLeft: '15px', paddingLeft: '3px' }}
    >
      {content}
    </div>
  );
}

function DeleteConfirm({
  open,
  onOpenChange,
  onConfirm,
  name,
  kind,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
  name: string;
  kind: 'file' | 'folder';
}): React.JSX.Element {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {kind}?</AlertDialogTitle>
          <AlertDialogDescription>
            <span className="font-mono text-mneme-cyan">{name}</span> will be permanently deleted
            from disk. {kind === 'folder' && 'All contents inside the folder will also be removed.'}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            className="bg-destructive text-destructive-foreground hover:brightness-110"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
