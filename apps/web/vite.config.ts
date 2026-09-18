import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import { TanStackRouterVite } from '@tanstack/router-plugin/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    TanStackRouterVite({
      target: 'react',
      autoCodeSplitting: true,
      routesDirectory: 'src/routes',
      generatedRouteTree: 'src/routeTree.gen.ts',
    }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@mneme/shared': path.resolve(__dirname, '../../packages/shared/src/index.ts'),
    },
  },
  // The Tauri dev shell loads the frontend from this port; API calls go through
  // Tauri IPC (invoke), not HTTP — so there is no dev proxy.
  server: {
    port: 5174,
    strictPort: true,
  },
});
