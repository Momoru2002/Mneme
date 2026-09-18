import { useNavigate } from '@tanstack/react-router';
import { Command } from 'cmdk';
import type React from 'react';
import { useState } from 'react';
import { toast } from 'sonner';
import { useSearch } from '../../lib/search.ts';
import { useTheme } from '../../lib/theme.ts';
import { useActivateWell, useWells } from '../../lib/wells.ts';
import { useUiStore } from '../../stores/ui.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
import { Dialog, DialogContent, DialogTitle } from '../ui/dialog.tsx';

type Mode = 'commands' | 'files';

type Props = {
  open: boolean;
  mode: Mode;
  onOpenChange: (open: boolean) => void;
};

/**
 * ⌘K command palette + ⌘P quick-open (IA-NAV-1). Quick-open filters server-side
 * via the existing search command (filenames + content); commands are filtered
 * client-side by cmdk. Every rail destination is mirrored here.
 */
export function CommandPalette({ open, mode, onOpenChange }: Props): React.JSX.Element {
  const navigate = useNavigate();
  const wells = useWells();
  const activate = useActivateWell();
  const { setTheme, theme } = useTheme();
  const openInActiveGroup = useWorkspaceStore((s) => s.openInActiveGroup);
  const closeAllTabs = useWorkspaceStore((s) => s.closeAllTabs);
  const requestNewFile = useUiStore((s) => s.requestNewFile);
  const requestSettingsOpen = useUiStore((s) => s.requestSettingsOpen);
  const setSidebarView = useUiStore((s) => s.setSidebarView);
  const setSidebarCollapsed = useUiStore((s) => s.setSidebarCollapsed);
  const activeWellId = wells.data?.activeWellId ?? null;

  const [query, setQuery] = useState('');
  const search = useSearch(activeWellId, mode === 'files' ? query : '', false);

  const close = () => {
    onOpenChange(false);
    setQuery('');
  };
  const run = (fn: () => void) => {
    close();
    fn();
  };

  return (
    <Dialog open={open} onOpenChange={(o) => (o ? onOpenChange(true) : close())}>
      <DialogContent className="max-w-xl p-0">
        <DialogTitle className="sr-only">Command palette</DialogTitle>
        <Command shouldFilter={mode === 'commands'} label="Command palette">
          <Command.Input
            value={query}
            onValueChange={setQuery}
            placeholder={mode === 'files' ? 'Search files…' : 'Type a command…'}
            className="w-full border-b border-border bg-transparent px-4 py-3 text-sm outline-none placeholder:text-muted-foreground"
          />
          <Command.List className="max-h-80 overflow-auto p-2">
            <Command.Empty className="px-2 py-6 text-center text-sm text-muted-foreground">
              {mode === 'files' && !activeWellId
                ? 'No well selected — choose a well to search files.'
                : 'No results.'}
            </Command.Empty>

            {mode === 'files' &&
              (search.data?.results ?? []).map((r) => (
                <Command.Item
                  key={r.path}
                  value={r.path}
                  onSelect={() =>
                    run(() => {
                      if (activeWellId) openInActiveGroup({ wellId: activeWellId, path: r.path });
                    })
                  }
                  className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm aria-selected:bg-muted"
                >
                  {r.path}
                </Command.Item>
              ))}

            {mode === 'commands' && (
              <>
                <Command.Group heading="Editor">
                  <Item
                    onSelect={() =>
                      run(() => {
                        if (!activeWellId) {
                          toast.error('Select a well first');
                          return;
                        }
                        requestNewFile();
                      })
                    }
                  >
                    New File
                  </Item>
                  <Item onSelect={() => run(() => closeAllTabs())}>Close All Tabs</Item>
                </Command.Group>

                <Command.Group heading="Navigate">
                  <Item
                    onSelect={() =>
                      run(() => {
                        navigate({ to: '/' });
                        setSidebarView('search');
                        setSidebarCollapsed(false);
                      })
                    }
                  >
                    Open Search
                  </Item>
                  <Item onSelect={() => run(() => navigate({ to: '/templates' }))}>
                    Open Templates
                  </Item>
                  <Item onSelect={() => run(() => requestSettingsOpen())}>Open Settings</Item>
                </Command.Group>

                <Command.Group heading="Wells">
                  {wells.data?.wells.map((w) => (
                    <Item key={w.id} onSelect={() => run(() => activate.mutate(w.id))}>
                      Switch to well: {w.name}
                    </Item>
                  ))}
                </Command.Group>

                <Command.Group heading="Preferences">
                  <Item onSelect={() => run(() => setTheme(theme === 'dark' ? 'light' : 'dark'))}>
                    Toggle theme
                  </Item>
                </Command.Group>
              </>
            )}
          </Command.List>
        </Command>
      </DialogContent>
    </Dialog>
  );
}

function Item({
  onSelect,
  children,
}: {
  onSelect: () => void;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <Command.Item
      onSelect={onSelect}
      className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm aria-selected:bg-muted"
    >
      {children}
    </Command.Item>
  );
}
