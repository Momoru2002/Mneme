import { SplitSquareHorizontal, SplitSquareVertical, X } from 'lucide-react';
import type React from 'react';
import { useShallow } from 'zustand/shallow';
import { cn } from '../../lib/cn.ts';
import { docKeyOf, useDocumentsStore } from '../../stores/documents.ts';
import type { GroupNode } from '../../stores/workspace.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
import { ViewModeToggle } from '../editor/ViewModeToggle.tsx';
import { Button } from '../ui/button.tsx';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '../ui/context-menu.tsx';
import { FileEditor } from './FileEditor.tsx';

type Props = { group: GroupNode };

function basename(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1).replace(/\.md$/, '');
}

export function EditorGroup({ group }: Props): React.JSX.Element {
  const activeGroupId = useWorkspaceStore((s) => s.activeGroupId);
  const setActiveGroup = useWorkspaceStore((s) => s.setActiveGroup);
  const setActiveTab = useWorkspaceStore((s) => s.setActiveTab);
  const closeTab = useWorkspaceStore((s) => s.closeTab);
  const closeGroup = useWorkspaceStore((s) => s.closeGroup);
  const root = useWorkspaceStore((s) => s.root);
  const setGroupViewMode = useWorkspaceStore((s) => s.setGroupViewMode);
  const splitGroup = useWorkspaceStore((s) => s.splitGroup);
  const closeAllTabs = useWorkspaceStore((s) => s.closeAllTabs);
  const isRoot = root.type === 'group' && root.id === group.id;

  const onCloseTab = (i: number) => {
    if (group.tabs.length === 1 && !isRoot) {
      closeGroup(group.id); // collapsing the split is friendlier than an empty pane
    } else {
      closeTab(group.id, i);
    }
  };
  const dirtyByKey = useDocumentsStore(
    useShallow((s) => {
      const result: Record<string, boolean> = {};
      for (const t of group.tabs) {
        const k = docKeyOf(t.wellId, t.path);
        result[k] = s.docs[k]?.dirty ?? false;
      }
      return result;
    }),
  );
  const isActiveGroup = activeGroupId === group.id;
  const active = group.activeTab !== null ? group.tabs[group.activeTab] : null;

  return (
    // The group container's onMouseDown focuses it for "open here" routing; inner
    // controls stop propagation. (Biome 1.9.4 has no static-element-interactions
    // rule, so no suppression is needed here — cf. Sidebar's resize-handle div.)
    <div
      className={cn(
        'flex h-full flex-col overflow-hidden',
        isActiveGroup && 'ring-1 ring-inset ring-mneme-cyan/40',
      )}
      onMouseDown={() => setActiveGroup(group.id)}
    >
      <div className="flex items-stretch border-b border-border bg-muted/40">
        <div className="flex min-w-0 flex-1 items-stretch overflow-x-auto">
          {group.tabs.map((t, i) => {
            const key = docKeyOf(t.wellId, t.path);
            const dirty = dirtyByKey[key] ?? false;
            const isActive = group.activeTab === i;
            return (
              <ContextMenu key={key}>
                <ContextMenuTrigger asChild>
                  <div
                    className={cn(
                      'group/tab flex min-w-0 max-w-[12rem] items-center gap-1.5 border-r border-border px-3 py-1.5 text-xs',
                      isActive
                        ? 'bg-background text-foreground'
                        : 'text-muted-foreground hover:bg-background/50',
                    )}
                  >
                    <button
                      type="button"
                      className="min-w-0 flex-1 truncate text-left"
                      title={t.path}
                      onClick={() => setActiveTab(group.id, i)}
                      onAuxClick={(e) => {
                        if (e.button === 1) onCloseTab(i); // middle-click closes
                      }}
                    >
                      {basename(t.path)}
                    </button>
                    <span
                      className={cn(
                        'h-1.5 w-1.5 shrink-0 rounded-full bg-mneme-cyan',
                        dirty ? 'opacity-100' : 'opacity-0',
                      )}
                      aria-hidden="true"
                    />
                    <button
                      type="button"
                      aria-label={`Close ${basename(t.path)}`}
                      className="shrink-0 rounded p-0.5 opacity-0 hover:bg-muted group-hover/tab:opacity-100"
                      onClick={() => onCloseTab(i)}
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                </ContextMenuTrigger>
                <ContextMenuContent>
                  <ContextMenuItem onSelect={() => onCloseTab(i)}>Close</ContextMenuItem>
                  <ContextMenuItem onSelect={() => closeAllTabs()}>Close All Tabs</ContextMenuItem>
                </ContextMenuContent>
              </ContextMenu>
            );
          })}
        </div>
        <div className="flex shrink-0 items-center gap-1 px-2">
          <ViewModeToggle value={group.viewMode} onChange={(m) => setGroupViewMode(group.id, m)} />
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            aria-label="Split right"
            title="Split right"
            onClick={() => splitGroup(group.id, 'row')}
          >
            <SplitSquareHorizontal className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            aria-label="Split down"
            title="Split down"
            onClick={() => splitGroup(group.id, 'col')}
          >
            <SplitSquareVertical className="h-3.5 w-3.5" />
          </Button>
          {!isRoot && (
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              aria-label="Close pane"
              title="Close pane"
              onClick={() => closeGroup(group.id)}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>
      </div>

      <div className="min-h-0 flex-1">
        {active ? (
          <FileEditor wellId={active.wellId} path={active.path} viewMode={group.viewMode} />
        ) : (
          <div className="flex h-full items-center justify-center p-6">
            <p className="text-sm text-muted-foreground italic">No file open in this pane.</p>
          </div>
        )}
      </div>
    </div>
  );
}
