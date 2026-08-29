import { useCallback, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../core/tauri/bindings';
import { useActiveGame } from '../../dashboard/hooks/useActiveGame';
import { applyDiskReconcileResult } from '../../file-watcher/hooks/useFileWatcher';
import { openObjectClassificationWizard } from '../../import-batches/classificationLauncher';

export function useScanReviewFlow(objectIds: string[]) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const [isSyncing, setIsSyncing] = useState(false);
  const isSyncingRef = useRef(false);

  const handleSync = useCallback(() => {
    if (!activeGame || objectIds.length === 0) return;
    openObjectClassificationWizard({ gameId: activeGame.id, objectIds });
  }, [activeGame, objectIds]);

  const handleBackgroundSync = useCallback(async () => {
    if (!activeGame || isSyncingRef.current) return;
    isSyncingRef.current = true;
    setIsSyncing(true);
    try {
      const result = await commands.reconcileDiskStateCmd(
        activeGame.id,
        'ManualRepair',
        null,
        true,
      );
      applyDiskReconcileResult(result, queryClient, activeGame);
    } catch (error) {
      console.error('Background index repair failed:', error);
    } finally {
      isSyncingRef.current = false;
      setIsSyncing(false);
    }
  }, [activeGame, queryClient]);

  return {
    isSyncing,
    handleSync,
    handleBackgroundSync,
  };
}
