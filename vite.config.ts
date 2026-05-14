import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath, URL } from 'node:url';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // CSS transformation is handled automatically by @tailwindcss/vite
  // using the lightningcss version enforced in pnpm.overrides.
  build: {
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (id.includes('@tauri-apps')) return 'vendor-tauri';
            if (id.includes('lucide-react')) return 'vendor-icons';
            if (id.includes('framer-motion') || id.includes('motion')) return 'vendor-motion';
            if (id.includes('@tanstack') || id.includes('query-core')) return 'vendor-query';
            if (id.includes('zustand')) return 'vendor-state';

            // Core libraries: only include the actual React core and scheduler
            if (
              id.includes('node_modules/react/') ||
              id.includes('node_modules/react-dom/') ||
              id.includes('node_modules/scheduler/')
            ) {
              return 'vendor-core';
            }

            return 'vendor-utils';
          }
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,

  // Vitest Configuration
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/setupTests.ts',
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    css: true,
    deps: {
      optimizer: {
        web: {
          include: ['@tauri-apps/plugin-fs'],
        },
      },
    },
    alias: {
      '@tauri-apps/plugin-fs': fileURLToPath(
        new URL('./src/testing/mocks/tauri-plugin-fs.ts', import.meta.url),
      ),
    },
  },

  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
});
