import { Outlet, createFileRoute } from '@tanstack/react-router';
import type React from 'react';
import { AppShell } from '../components/layout/AppShell.tsx';
import { useFileWatcher } from '../lib/watcher.ts';
import { useWells } from '../lib/wells.ts';

export const Route = createFileRoute('/_app')({
  component: AppLayout,
});

function AppLayout(): React.JSX.Element {
  const wells = useWells();
  useFileWatcher(wells.data?.activeWellId ?? null);
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
