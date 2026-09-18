import { useQueryClient } from '@tanstack/react-query';
import {
  ChevronLeft,
  FilePlus,
  FolderPlus,
  FolderTree,
  ListCollapse,
  RefreshCw,
} from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useWells } from '../../lib/wells.ts';
import { useFileTreeStore } from '../../stores/file-tree.ts';
import { useUiStore } from '../../stores/ui.ts';
import { CreateInFolderDialog } from '../file-tree/CreateInFolderDialog.tsx';
import { FileTree } from '../file-tree/FileTree.tsx';
import { MoveDialog, type MoveTarget } from '../file-tree/MoveDialog.tsx';
import { Button } from '../ui/button.tsx';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu.tsx';
import { ScrollArea } from '../ui/scroll-area.tsx';
import { Separator } from '../ui/separator.tsx';

export function FilesPanel(): React.JSX.Element {
  const setSidebarCollapsed = useUiStore((s) => s.setSidebarCollapsed);
  const collapseAll = useFileTreeStore((s) => s.collapseAll);
  const [createOpen, setCreateOpen] = useState<{
    kind: 'file' | 'folder';
    folderPath: string;
  } | null>(null);
  const [moveTarget, setMoveTarget] = useState<MoveTarget | null>(null);
  const wells = useWells();
  const qc = useQueryClient();
  const activeWellId = wells.data?.activeWellId ?? null;

  return (
    <>
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Files
        </span>
        <div className="flex items-center">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" disabled={!activeWellId} aria-label="New">
                <FilePlus className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => setCreateOpen({ kind: 'file', folderPath: '' })}>
                <FilePlus className="h-3.5 w-3.5" /> New File
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setCreateOpen({ kind: 'folder', folderPath: '' })}>
                <FolderPlus className="h-3.5 w-3.5" /> New Folder
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button
            variant="ghost"
            size="icon"
            onClick={() =>
              activeWellId && qc.invalidateQueries({ queryKey: ['tree', activeWellId] })
            }
            disabled={!activeWellId}
            aria-label="Refresh"
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => activeWellId && collapseAll(activeWellId)}
            disabled={!activeWellId}
            aria-label="Collapse all"
          >
            <ListCollapse className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSidebarCollapsed(true)}
            aria-label="Hide sidebar"
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
        </div>
      </div>
      <Separator />
      <ScrollArea className="flex-1">
        {!activeWellId ? (
          <div className="flex flex-col gap-2 p-6 text-center">
            <FolderTree className="mx-auto h-8 w-8 text-muted-foreground/40" />
            <p className="text-xs text-muted-foreground">
              Choose or add a well from the header to see files here.
            </p>
          </div>
        ) : (
          <FileTree
            wellId={activeWellId}
            onCreateInFolder={(folderPath, kind) => setCreateOpen({ kind, folderPath })}
            onMove={setMoveTarget}
          />
        )}
      </ScrollArea>

      <CreateInFolderDialog
        wellId={activeWellId}
        open={createOpen != null}
        onOpenChange={(o) => !o && setCreateOpen(null)}
        folderPath={createOpen?.folderPath ?? ''}
        kind={createOpen?.kind ?? 'file'}
      />

      {activeWellId && (
        <MoveDialog
          wellId={activeWellId}
          target={moveTarget}
          onOpenChange={(o) => !o && setMoveTarget(null)}
        />
      )}
    </>
  );
}
