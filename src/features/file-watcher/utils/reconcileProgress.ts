import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { DiskReconcileProgress } from '../../../shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';

/** Keeps event-only scan progress out of the long-lived reconcile coordinator. */
export function useDiskReconcileProgress(gameId: string | null): void {
  const setDiskReconcileProgress = useAppStore((state) => state.setDiskReconcileProgress);
  const markDiskReconcilePending = useAppStore((state) => state.markDiskReconcilePending);

  useEffect(() => {
    if (!gameId) {
      return;
    }

    let active = true;
    let previousProgress: DiskReconcileProgress | null = null;
    const unlistenPromise = listen<DiskReconcileProgress>(
      'disk_reconcile:progress',
      ({ payload }) => {
        if (!active || payload.game_id !== gameId) {
          return;
        }
        if (payload.phase === 'Completed' || payload.phase === 'Failed') {
          // Authority validation can retry a completed pass with the same run ID.
          previousProgress = null;
          setDiskReconcileProgress(gameId, null);
          if (payload.phase === 'Failed') {
            markDiskReconcilePending(gameId, true);
          }
          return;
        }
        if (
          previousProgress?.run_id === payload.run_id &&
          previousProgress.phase === payload.phase &&
          previousProgress.total_units === payload.total_units &&
          payload.completed_units < previousProgress.completed_units
        ) {
          return;
        }
        previousProgress = payload;
        setDiskReconcileProgress(gameId, payload);
      },
    ).catch((error: unknown) => {
      active = false;
      console.error('Failed to register disk reconcile progress listener', error);
    });
    return () => {
      active = false;
      void unlistenPromise
        .then((unlisten) => unlisten?.())
        .catch((error: unknown) => {
          console.error('Failed to remove disk reconcile progress listener', error);
        });
    };
  }, [gameId, markDiskReconcilePending, setDiskReconcileProgress]);
}
