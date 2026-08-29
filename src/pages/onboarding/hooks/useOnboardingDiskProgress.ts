import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { DiskReconcileProgress } from '../../../shared/api/tauri/bindings';
import type { GameConfig } from '@/entities/game/model/game';

export function useOnboardingDiskProgress(
  isIndexing: boolean,
  games: GameConfig[],
): DiskReconcileProgress | null {
  const [diskProgress, setDiskProgress] = useState<DiskReconcileProgress | null>(null);

  useEffect(() => {
    if (!isIndexing) {
      setDiskProgress(null);
      return;
    }

    const gameIds = new Set(games.map((game) => game.id));
    const unlistenPromise = listen<DiskReconcileProgress>('disk_reconcile:progress', (event) => {
      if (gameIds.has(event.payload.game_id)) {
        setDiskProgress(event.payload);
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [games, isIndexing]);

  return diskProgress;
}
