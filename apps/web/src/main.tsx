import { RouterProvider, createRouter } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from './components/ErrorBoundary.tsx';
import { Toaster } from './components/ui/sonner.tsx';
import { TooltipProvider } from './components/ui/tooltip.tsx';
import { bootstrapToken, isTauri } from './lib/web-mode.ts';
import { routeTree } from './routeTree.gen.ts';
import { queryClient } from './routes/__root.tsx';
import './styles/globals.css';

// Read the #token=<hex> fragment (injected by "Open in browser") into
// localStorage and strip it from the URL before any query fires (W7).
bootstrapToken();

// Register the app-shell service worker only in a real browser tab (web
// mode) — never inside the Tauri webview, which has no use for it and where
// `serviceWorker` may not even be exposed. Browsers also only allow service
// workers in a "secure context" (HTTPS, or the special-cased `localhost`) —
// so this activates when web mode is opened via 127.0.0.1/localhost, but a
// LAN IP over plain HTTP (e.g. from a phone) won't register one; that's a
// browser platform rule, not something toggleable here, and it doesn't
// block "Add to Home Screen" (which uses the <link rel="manifest"> instead).
if (!isTauri() && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      /* offline app-shell caching is a nice-to-have, not required */
    });
  });
}

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
