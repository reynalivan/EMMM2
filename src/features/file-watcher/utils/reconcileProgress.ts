import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { DiskReconcileProgress } from '../../../core/tauri/bindings';
import { useAppStore } from '../../../stores/useAppStore';

/** Keeps event-only scan progress out of the long-lived reconcile coordinator. */
export function useDiskReconcileProgress(gameId: string | null): void {
  const setDiskReconcileProgress = useAppStore((state) => state.setDiskReconcileProgress);
  const markDiskReconcilePending = useAppStore((state) => state.markDiskReconcilePending);

  useEffect(() => {
    if (!gameId) {
      return;
    }

    const unlistenPromise = listen<DiskReconcileProgress>('disk_reconcile:progress', (event) => {
      if (event.payload.game_id === gameId) {
        if (event.payload.phase === 'Completed' || event.payload.phase === 'Failed') {
          setDiskReconcileProgress(gameId, null);
          if (event.payload.phase === 'Failed') {
            markDiskReconcilePending(gameId, true);
          }
          return;
        }
        setDiskReconcileProgress(gameId, event.payload);
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [gameId, markDiskReconcilePending, setDiskReconcileProgress]);
}
