import { formatAppError } from '../../../shared/lib/appError';
import { create } from 'zustand';
import { Channel } from '@tauri-apps/api/core';
import { commands } from '../../../shared/api/tauri/bindings';
import type { AppUpdateInfo, AppUpdateProgress } from '../../../shared/api/tauri/bindings.gen';

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

interface UpdaterOperation {
  readonly kind: 'idle' | 'checking' | 'installing';
}

interface AppUpdaterState {
  operation: UpdaterOperation;
  update: AppUpdateInfo | null;
  progress: UpdateProgress | null;
  error: string | null;
  hasChecked: boolean;
  checkForUpdate: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  dismiss: () => void;
}

const IDLE_OPERATION: UpdaterOperation = { kind: 'idle' };

const useAppUpdaterStore = create<AppUpdaterState>((set, get) => ({
  operation: IDLE_OPERATION,
  update: null,
  progress: null,
  error: null,
  hasChecked: false,

  checkForUpdate: async () => {
    if (get().operation.kind !== 'idle') return;
    const operation: UpdaterOperation = { kind: 'checking' };
    set({ operation, error: null, progress: null });
    try {
      const update = await commands.checkAppUpdate();
      if (get().operation !== operation) return;
      set({ update, hasChecked: true, operation: IDLE_OPERATION });
    } catch (e) {
      if (get().operation !== operation) return;
      set({ error: formatAppError(e), operation: IDLE_OPERATION });
    }
  },

  downloadAndInstall: async () => {
    if (!get().update || get().operation.kind !== 'idle') return;
    const operation: UpdaterOperation = { kind: 'installing' };
    set({ operation, error: null, progress: { downloaded: 0, total: null } });
    try {
      const progressChannel = new Channel<AppUpdateProgress>();
      let downloadFinished = false;
      progressChannel.onmessage = (event) => {
        if (get().operation !== operation || downloadFinished) return;
        if (event.event === 'Started') {
          set({ progress: { downloaded: 0, total: event.data.contentLength ?? null } });
        } else if (event.event === 'Progress') {
          const previous = get().progress;
          set({
            progress: {
              downloaded: (previous?.downloaded ?? 0) + event.data.chunkLength,
              total: previous?.total ?? null,
            },
          });
        } else if (event.event === 'Finished') {
          downloadFinished = true;
          const previous = get().progress;
          set({
            progress: {
              downloaded: previous?.total ?? previous?.downloaded ?? 0,
              total: previous?.total ?? null,
            },
          });
        }
      };
      await commands.installAppUpdate(progressChannel);
      // A successful install exits/restarts the app; retain admission until then.
    } catch (e) {
      if (get().operation !== operation) return;
      set({ error: formatAppError(e), operation: IDLE_OPERATION });
    }
  },

  dismiss: () => {
    if (get().operation.kind !== 'idle') return;
    set({ update: null, progress: null, error: null, hasChecked: false });
  },
}));

export function useAppUpdater() {
  const {
    operation,
    update,
    progress,
    error,
    hasChecked,
    checkForUpdate,
    downloadAndInstall,
    dismiss,
  } = useAppUpdaterStore();
  return {
    update,
    isChecking: operation.kind === 'checking',
    isInstalling: operation.kind === 'installing',
    progress,
    error,
    hasChecked,
    checkForUpdate,
    downloadAndInstall,
    dismiss,
  };
}
