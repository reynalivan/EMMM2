import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../dashboard/hooks/useActiveGame';
import { useAppStore } from '../../../stores/useAppStore';
import { applyDiskReconcileResult } from '../../file-watcher/hooks/useFileWatcher';

export function useApplyFolderConflictActionResult() {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  return (result: Parameters<typeof applyDiskReconcileResult>[0]) => {
    if (useAppStore.getState().activeGameId !== result.game_id) {
      useAppStore.getState().setFolderConflicts(result.game_id, result.folder_conflicts);
      useAppStore.getState().setRenameConfirmations(result.game_id, result.rename_confirmations);
      return false;
    }
    applyDiskReconcileResult(
      result,
      queryClient,
      activeGame?.id === result.game_id ? activeGame : null,
    );
    return true;
  };
}
