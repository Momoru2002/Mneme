import { Check, ChevronsUpDown, Plus, Settings2 } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useActivateWell, useWells } from '../../lib/wells.ts';
import { useUiStore } from '../../stores/ui.ts';
import { Button } from '../ui/button.tsx';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu.tsx';
import { AddWellDialog } from '../well/AddWellDialog.tsx';

export function WellSelector(): React.JSX.Element {
  const [addOpen, setAddOpen] = useState(false);
  const wells = useWells();
  const activate = useActivateWell();
  const requestSettingsOpen = useUiStore((s) => s.requestSettingsOpen);

  const active = wells.data?.wells.find((v) => v.id === wells.data?.activeWellId);
  const triggerLabel = active?.name ?? (wells.data?.wells.length ? 'Select well' : 'No well');

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="min-w-[180px] justify-between">
            <span className="truncate">{triggerLabel}</span>
            <ChevronsUpDown className="h-3.5 w-3.5 opacity-60" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="min-w-[240px]">
          <DropdownMenuLabel>Wells</DropdownMenuLabel>
          {wells.data?.wells.length === 0 && (
            <DropdownMenuItem disabled>
              <span className="text-muted-foreground italic">No wells registered yet</span>
            </DropdownMenuItem>
          )}
          {wells.data?.wells.map((v) => (
            <DropdownMenuItem
              key={v.id}
              onClick={() => activate.mutate(v.id)}
              className="flex items-center justify-between"
            >
              <div className="flex flex-col">
                <span>{v.name}</span>
                <span className="text-xs text-muted-foreground font-mono truncate max-w-[200px]">
                  {v.path}
                </span>
              </div>
              {wells.data?.activeWellId === v.id && <Check className="h-4 w-4 text-mneme-cyan" />}
            </DropdownMenuItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={() => setAddOpen(true)}>
            <Plus className="h-4 w-4" /> Add Well…
          </DropdownMenuItem>
          {(wells.data?.wells.length ?? 0) > 0 && (
            <DropdownMenuItem onClick={() => requestSettingsOpen('wells')}>
              <Settings2 className="h-4 w-4" /> Manage Wells…
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      <AddWellDialog open={addOpen} onOpenChange={setAddOpen} />
    </>
  );
}
