import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../core/tauri/bindings';
import { applyDiskReconcileResult } from '../../file-watcher/hooks/useFileWatcher';
import { toast } from '../../../stores/useToastStore';
import type { GameConfig } from '../../../types/game';
import { thumbnailKeys } from '../../dashboard/hooks/useThumbnail';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildPathInvalidationDescriptor,
  buildPathRewriteDescriptor,
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
} from '../../workspace-runtime/optimistic/descriptorBuilders';
import { formatAppError } from '../../../core/lib/appError';
import { openFileInUseRetryDialog } from '../../../shared/hooks/fileInUseRetry';
import { publishCollectionReferenceImpact } from '../../collections/hooks/collectionReferenceImpact';
import { notifyCommittedMutationSyncWarning } from '../../../core/lib/committedMutationWarning';

async function runDiskRepairRecovery(
  queryClient: ReturnType<typeof useQueryClient>,
  gameId: string | null,
) {
  if (!gameId) {
    return;
  }

  toast.info('Syncing changes from disk...', 3000);
  try {
    const result = await commands.reconcileDiskStateCmd(gameId, 'ManualRepair', null, true);
    const settings = await commands.getSettings();
    const activeGame: GameConfig | null = settings.games.find((game) => game.id === gameId) ?? null;
    applyDiskReconcileResult(result, queryClient, activeGame);
    toast.success('Sync complete', 2000);
  } catch (error) {
    console.error('Disk repair recovery failed:', error);
    toast.error('Sync failed', 3000);
  }
}

/** A missing path means the DB drifted from disk — repair instead of erroring. */
function isMissingOnDisk(message: string): boolean {
  const lowered = message.toLowerCase();
  return lowered.includes('not found') || lowered.includes('os error 2');
}

/** Structure + conflict republish shared by every single-folder mutation. */
async function publishFolderStructureChange(queryClient: QueryClient): Promise<void> {
  await publishRuntimeDescriptor(
    queryClient,
    buildRuntimeMutationDescriptor('workspaceStructure'),
    'active',
  );
  await publishRuntimeDescriptor(
    queryClient,
    buildRuntimeMutationDescriptor('folderConflictState'),
    'none',
  );
}

export function useRenameMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: { folderPath: string; newName: string; gameId: string }) =>
      commands.renameModFolder(params.folderPath, params.newName, params.gameId),
    onSuccess: async (result, variables) => {
      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor([thumbnailKeys.folder(variables.folderPath)], []),
      );
      applyRuntimeEffects(
        queryClient,
        buildPathRewriteDescriptor(variables.folderPath, result.new_path, []),
      );
      await publishFolderStructureChange(queryClient);
      await publishCollectionReferenceImpact(queryClient, result.collection_impact);
      notifyCommittedMutationSyncWarning(result);
    },
    onError: (error, variables) => {
      if (openFileInUseRetryDialog(error, variables, mutation.mutate)) {
        return;
      }

      const errorMessage = formatAppError(error);
      if (isMissingOnDisk(errorMessage)) {
        void runDiskRepairRecovery(queryClient, variables.gameId);
        return;
      }

      toast.error(`Rename failed: ${errorMessage}`);
    },
  });

  return mutation;
}

export function useDeleteMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    // `gameId` names the mods root the path must sit inside; the backend
    // refuses to trash anything outside it, so it is no longer optional.
    mutationFn: (params: { path: string; gameId: string }) =>
      commands.deleteMod(params.path, params.gameId),
    onSuccess: async (result, variables) => {
      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor([thumbnailKeys.folder(variables.path)], []),
      );
      applyRuntimeEffects(queryClient, buildPathInvalidationDescriptor(variables.path, []));
      await publishFolderStructureChange(queryClient);
      await publishCollectionReferenceImpact(queryClient, result.collection_impact);
      notifyCommittedMutationSyncWarning(result);
    },
    onError: (error, variables) => {
      if (openFileInUseRetryDialog(error, variables, mutation.mutate)) {
        return;
      }

      const errorMessage = formatAppError(error);
      if (isMissingOnDisk(errorMessage)) {
        void runDiskRepairRecovery(queryClient, variables.gameId ?? null);
        return;
      }

      toast.error(`Delete failed: ${errorMessage}`);
    },
  });

  return mutation;
}
