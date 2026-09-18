import { createFileRoute } from '@tanstack/react-router';
import { FolderPlus } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { EditorWorkbench } from '../components/file-view/EditorWorkbench.tsx';
import { Button } from '../components/ui/button.tsx';
import { AddWellDialog } from '../components/well/AddWellDialog.tsx';
import { useWells } from '../lib/wells.ts';

export const Route = createFileRoute('/_app/')({
  component: WelcomePage,
});

function WelcomePage(): React.JSX.Element {
  const wells = useWells();
  const [addOpen, setAddOpen] = useState(false);

  const hasWells = (wells.data?.wells.length ?? 0) > 0;
  const active = wells.data?.wells.find((v) => v.id === wells.data?.activeWellId) ?? null;

  if (hasWells && active) {
    return <EditorWorkbench wellId={active.id} />;
  }

  return (
    <div className="flex h-full w-full flex-col items-center justify-center gap-6 p-6 text-foreground">
      <h1 className="text-2xl font-extrabold tracking-widest text-mneme-cyan">Welcome to Mneme</h1>

      {!hasWells && (
        <div className="flex flex-col items-center gap-3 rounded-lg border border-border bg-card p-8 max-w-md text-center">
          <FolderPlus className="h-12 w-12 text-mneme-cyan" />
          <h2 className="text-lg font-semibold">No well yet</h2>
          <p className="text-sm text-muted-foreground">
            Mneme keeps your notes on disk. Point it at an Obsidian-style folder of markdown files
            to begin.
          </p>
          <Button onClick={() => setAddOpen(true)} className="mt-2">
            Add your first well
          </Button>
        </div>
      )}

      {hasWells && !active && (
        <p className="text-sm text-muted-foreground italic">
          Choose a well from the header dropdown.
        </p>
      )}

      <AddWellDialog open={addOpen} onOpenChange={setAddOpen} />
    </div>
  );
}
