import { useCallback } from 'react';
import type { GameConfig } from '@/entities/game/model/game';
import { openObjectClassificationWizard } from '../../import-batches/classificationLauncher';

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
