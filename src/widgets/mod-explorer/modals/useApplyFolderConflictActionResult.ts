import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '@/entities/game';
import { useAppStore } from '@/app/store';
import { applyDiskReconcileResult } from '@/features/file-watcher';
import type { DiskReconcileResult } from '../../../shared/api/tauri/bindings';

function preserveRemainingConflictGroups(
  result: DiskReconcileResult,
  currentGroups: DiskReconcileResult['folder_conflicts'],
  resolvedGroupId: string | undefined,
): DiskReconcileResult {
  if (
    result.status !== 'Applied' ||
    result.folder_conflicts.length > 0 ||
    resolvedGroupId === undefined
  ) {
    return result;
  }

  const remainingGroups = currentGroups.filter((group) => group.group_id !== resolvedGroupId);
  if (remainingGroups.length === 0) {
    return result;
  }

  return {
    ...result,
    status: 'AppliedWithFolderConflicts',
    folder_conflicts: remainingGroups,
  };
}

export function useApplyFolderConflictActionResult() {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  return (result: Parameters<typeof applyDiskReconcileResult>[0], resolvedGroupId?: string) => {
    const currentGroups = useAppStore.getState().folderConflictsByGame[result.game_id] ?? [];
    const effectiveResult = preserveRemainingConflictGroups(result, currentGroups, resolvedGroupId);
    if (useAppStore.getState().activeGameId !== result.game_id) {
      useAppStore.getState().setFolderConflicts(result.game_id, effectiveResult.folder_conflicts);
      useAppStore
        .getState()
        .setRenameConfirmations(result.game_id, effectiveResult.rename_confirmations);
      return false;
    }
    applyDiskReconcileResult(
      effectiveResult,
      queryClient,
      activeGame?.id === effectiveResult.game_id ? activeGame : null,
    );
    return true;
  };
}
