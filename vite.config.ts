import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath, URL } from 'node:url';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(({ command, mode }) => {
  const isDemoBuild = mode === 'demo';

  // A demo build would embed development fixtures in a distributable artifact.
  // Fail before compilation rather than relying on a convention in CI or Tauri.
  if (command === 'build' && isDemoBuild) {
    throw new Error(
      'Demo mode is development-only. Use `pnpm dev:demo`, never `vite build --mode demo`.',
    );
  }

  const demoModule = (name: string) =>
    fileURLToPath(
      new URL(
        isDemoBuild ? `./src/demo/${name}.ts` : `./src/demo/runtime/${name}.disabled.ts`,
        import.meta.url,
      ),
    );

  return {
    plugins: [react(), tailwindcss()],
    define: {
      'import.meta.env.VITE_APP_MODE': JSON.stringify(isDemoBuild ? 'demo' : 'app'),
    },
    resolve: {
      alias: [
        { find: '@/demo/bootstrap', replacement: demoModule('bootstrap') },
        { find: '@/demo/commands', replacement: demoModule('commands') },
        { find: '@/demo/dashboard', replacement: demoModule('dashboard') },
        { find: '@/demo/game', replacement: demoModule('game') },
        { find: '@', replacement: fileURLToPath(new URL('./src', import.meta.url)) },
      ],
    },
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
              if (id.includes('recharts')) return 'vendor-charts';
              if (id.includes('@grafana') || id.includes('@opentelemetry')) {
                return 'vendor-observability';
              }
              if (id.includes('react-router')) return 'vendor-router';
              if (id.includes('i18next')) return 'vendor-i18n';
              if (id.includes('react-hook-form') || id.includes('@hookform')) return 'vendor-forms';
              if (id.includes('zod')) return 'vendor-validation';
              if (id.includes('@radix-ui') || id.includes('@floating-ui')) {
                return 'vendor-primitives';
              }
              if (id.includes('quick-liquid')) return 'vendor-liquid';

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
      setupFiles: './src/tests/setupTests.ts',
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
          new URL('./src/tests/testing/mocks/tauri-plugin-fs.ts', import.meta.url),
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
  };
});
