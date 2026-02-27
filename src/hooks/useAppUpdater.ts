import { useState, useCallback } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

export function useAppUpdater() {
  const [isChecking, setIsChecking] = useState(false);
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkForUpdate = useCallback(async () => {
    setIsChecking(true);
    setError(null);
    try {
      const found = await check();
      setUpdate(found);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsChecking(false);
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    if (!update) return;
    setIsInstalling(true);
    setError(null);
    setProgress({ downloaded: 0, total: null });
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          setProgress({ downloaded: 0, total: event.data.contentLength ?? null });
        } else if (event.event === 'Progress') {
          setProgress((prev) => ({
            downloaded: (prev?.downloaded ?? 0) + event.data.chunkLength,
            total: prev?.total ?? null,
          }));
        } else if (event.event === 'Finished') {
          setProgress((prev) => ({
            downloaded: prev?.total ?? prev?.downloaded ?? 0,
            total: prev?.total ?? null,
          }));
        }
      });
      // Restart the app after install
      await relaunch();
    } catch (e) {
      setError(String(e));
      setIsInstalling(false);
    }
  }, [update]);

  const dismiss = useCallback(() => {
    setUpdate(null);
    setProgress(null);
    setError(null);
  }, []);

  return {
    update,
    isChecking,
    isInstalling,
    progress,
    error,
    checkForUpdate,
    downloadAndInstall,
    dismiss,
  };
}
