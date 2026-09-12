import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { DiskReconcileProgress } from '../../../shared/api/tauri/bindings';
import type { GameConfig } from '@/entities/game';

export interface OnboardingDiskProgress {
  current: DiskReconcileProgress;
  completedRootsByGame: Record<string, string[]>;
}

export function useOnboardingDiskProgress(
  isIndexing: boolean,
  games: GameConfig[],
): OnboardingDiskProgress | null {
  const [diskProgress, setDiskProgress] = useState<OnboardingDiskProgress | null>(null);

  useEffect(() => {
    if (!isIndexing) {
      setDiskProgress(null);
      return;
    }

    const gameIds = new Set(games.map((game) => game.id));
    const unlistenPromise = listen<DiskReconcileProgress>('disk_reconcile:progress', (event) => {
      if (gameIds.has(event.payload.game_id)) {
        setDiskProgress((previous) => {
          const rootName = event.payload.current_root;
          const completedRoots = rootName
            ? new Set(previous?.completedRootsByGame[event.payload.game_id] ?? []).add(rootName)
            : null;
          return {
            current: event.payload,
            completedRootsByGame: completedRoots
              ? {
                  ...previous?.completedRootsByGame,
                  [event.payload.game_id]: [...completedRoots],
                }
              : (previous?.completedRootsByGame ?? {}),
          };
        });
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [games, isIndexing]);

  return diskProgress;
}
