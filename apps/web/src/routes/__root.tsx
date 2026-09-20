import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Outlet, createRootRouteWithContext } from '@tanstack/react-router';
import type React from 'react';
import { AppLockGate } from '../components/AppLockGate.tsx';
import { WebAuthExpired } from '../components/WebAuthExpired.tsx';
import { SettingsBootstrap } from '../components/settings/SettingsBootstrap.tsx';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 30_000,
      refetchOnWindowFocus: false,
    },
  },
});

export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()({
  component: RootLayout,
});

function RootLayout(): React.JSX.Element {
  return (
    <QueryClientProvider client={queryClient}>
      <AppLockGate>
        <SettingsBootstrap />
        <WebAuthExpired />
        <Outlet />
      </AppLockGate>
    </QueryClientProvider>
  );
}

export { queryClient };
