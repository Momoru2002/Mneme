import { useQuery } from '@tanstack/react-query';
import { ChevronRight, Folder, FolderOpen, Home } from 'lucide-react';
import type React from 'react';
import { useEffect, useState } from 'react';
import { api } from '../../lib/api-client.ts';
import { useHostHome } from '../../lib/wells.ts';
import { Button } from '../ui/button.tsx';
import { ScrollArea } from '../ui/scroll-area.tsx';

type Props = {
  onSelect: (path: string) => void;
  selectedPath: string | null;
  /** Reports the folder currently being viewed (so it can be the default well). */
  onLocationChange?: (path: string) => void;
};

export function HostBrowser({
  onSelect,
  selectedPath,
  onLocationChange,
}: Props): React.JSX.Element {
  const home = useHostHome();
  const [current, setCurrent] = useState<string | null>(null);

  const browsePath = current ?? home.data?.path ?? null;

  // The folder you're currently viewing is the default well candidate — so you
  // can navigate to it and just click "Add well" without an explicit Choose.
  useEffect(() => {
    if (browsePath) onLocationChange?.(browsePath);
  }, [browsePath, onLocationChange]);

  const browse = useQuery({
    queryKey: ['wells', 'browse', browsePath],
    queryFn: () => {
      if (browsePath == null) throw new Error('No path to browse');
      return api.wells.browse(browsePath);
    },
    enabled: browsePath != null,
  });

  if (!home.data) {
    return <div className="text-sm text-muted-foreground p-4">Loading host home…</div>;
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Home className="h-3.5 w-3.5" />
        <span className="font-mono">{browse.data?.path ?? home.data.path}</span>
      </div>

      <ScrollArea className="h-72 rounded-md border border-border bg-background">
        <div className="p-1">
          {browse.data?.parent && (
            <button
              type="button"
              onClick={() => setCurrent(browse.data?.parent ?? null)}
              className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-muted"
            >
              <FolderOpen className="h-4 w-4 text-muted-foreground" />
              <span className="text-muted-foreground">..</span>
            </button>
          )}
          {browse.isLoading && (
            <div className="px-2 py-1.5 text-sm text-muted-foreground">Loading…</div>
          )}
          {browse.error && (
            <div className="px-2 py-1.5 text-sm text-destructive">
              {(browse.error as Error).message}
            </div>
          )}
          {browse.data?.entries.length === 0 && (
            <div className="px-2 py-1.5 text-sm text-muted-foreground italic">Empty folder</div>
          )}
          {browse.data?.entries.map((entry) => {
            const isSelected = entry.path === selectedPath;
            return (
              <div key={entry.path} className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => setCurrent(entry.path)}
                  disabled={!entry.hasChildren}
                  className="flex flex-1 items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-muted disabled:opacity-50 disabled:hover:bg-transparent"
                >
                  <Folder className="h-4 w-4 text-mneme-gold" />
                  <span>{entry.name}</span>
                  {entry.hasChildren && (
                    <ChevronRight className="ml-auto h-3.5 w-3.5 text-muted-foreground" />
                  )}
                </button>
                <Button
                  type="button"
                  variant={isSelected ? 'default' : 'ghost'}
                  size="sm"
                  onClick={() => onSelect(entry.path)}
                  className="text-xs"
                >
                  {isSelected ? 'Selected' : 'Choose'}
                </Button>
              </div>
            );
          })}
        </div>
      </ScrollArea>

      <div className="flex items-center justify-between text-xs">
        <span className="text-muted-foreground">Selected:</span>
        <span className="font-mono text-mneme-cyan">{selectedPath ?? '—'}</span>
      </div>

      {browsePath != null && browsePath !== home.data.path && (
        <div className="flex justify-between">
          <Button type="button" variant="ghost" size="sm" onClick={() => onSelect(browsePath)}>
            Choose current folder ({browsePath.split('/').pop()})
          </Button>
        </div>
      )}
    </div>
  );
}
