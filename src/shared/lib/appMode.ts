export type AppMode = 'app' | 'demo';

interface AppModeInput {
  dev: boolean;
  requestedMode: string | undefined;
}

export function resolveAppMode({ dev, requestedMode }: AppModeInput): AppMode {
  if (requestedMode === 'demo') {
    if (!dev) {
      throw new Error('Demo mode is only available from the Vite development server.');
    }
    return 'demo';
  }

  if (requestedMode === undefined || requestedMode === 'app') {
    return 'app';
  }

  throw new Error(`Unknown application mode: ${requestedMode}`);
}

export const appMode = resolveAppMode({
  dev: import.meta.env.DEV,
  requestedMode: import.meta.env.VITE_APP_MODE,
});

export const isDemoMode = appMode === 'demo';
