import { formatAppError } from '../../../shared/lib/appError';
import { useState, useCallback } from 'react';
import { Channel } from '@tauri-apps/api/core';
import { commands } from '../../../shared/api/tauri/bindings';
import type { AppUpdateInfo, AppUpdateProgress } from '../../../shared/api/tauri/bindings.gen';

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

export function useAppUpdater() {
  const [isChecking, setIsChecking] = useState(false);
  const [update, setUpdate] = useState<AppUpdateInfo | null>(null);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkForUpdate = useCallback(async () => {
    setIsChecking(true);
    setError(null);
    try {
      const found = await commands.checkAppUpdate();
      setUpdate(found);
    } catch (e) {
      setError(formatAppError(e));
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
      const progressChannel = new Channel<AppUpdateProgress>();
      progressChannel.onmessage = (event) => {
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
      };
      await commands.installAppUpdate(progressChannel);
    } catch (e) {
      setError(formatAppError(e));
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
