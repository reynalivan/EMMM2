import { useCallback } from 'react';
import type { GameConfig } from '@/entities/game';
import { openObjectClassificationWizard } from '@/features/import-batches/@x/workspace-runtime';

export function useSharedObjectSyncActions(activeGame: GameConfig | null) {
  const handleSyncWithDb = useCallback(
    async (objectId: string, _objectName: string) => {
      if (!activeGame) return;
      openObjectClassificationWizard({
        gameId: activeGame.id,
        objectIds: [objectId],
      });
    },
    [activeGame],
  );

  return { handleSyncWithDb };
}
