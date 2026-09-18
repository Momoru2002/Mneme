import path from 'node:path';
import { defineConfig } from 'vitest/config';

// Minimal config for unit tests (api-client contract tests mock the Tauri
// `invoke` boundary, so no DOM / vite plugins are needed). Kept separate from
// vite.config.ts to avoid pulling the router/tailwind plugins into the runner.
export default defineConfig({
  resolve: {
    alias: {
      '@mneme/shared': path.resolve(__dirname, '../../packages/shared/src/index.ts'),
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
