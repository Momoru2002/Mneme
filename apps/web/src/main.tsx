import { RouterProvider, createRouter } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from './components/ErrorBoundary.tsx';
import { Toaster } from './components/ui/sonner.tsx';
import { TooltipProvider } from './components/ui/tooltip.tsx';
import { bootstrapToken } from './lib/web-mode.ts';
import { routeTree } from './routeTree.gen.ts';
import { queryClient } from './routes/__root.tsx';
import './styles/globals.css';

// Read the #token=<hex> fragment (injected by "Open in browser") into
// localStorage and strip it from the URL before any query fires (W7).
bootstrapToken();

const router = createRouter({
  routeTree,
  context: { queryClient },
  defaultPreload: 'intent',
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('#root not found');

createRoot(rootEl).render(
  <StrictMode>
    <ErrorBoundary>
      <TooltipProvider>
        <RouterProvider router={router} />
        <Toaster richColors closeButton />
      </TooltipProvider>
    </ErrorBoundary>
  </StrictMode>,
);
