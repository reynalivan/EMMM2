import { useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import { applyDiskReconcileResult } from '../features/file-watcher/hooks';
import { toast } from '../stores/useToastStore';
import type { GameConfig } from '../types/game';
import { thumbnailKeys } from './useThumbnail';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../features/workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildPathInvalidationDescriptor,
  buildPathRewriteDescriptor,
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
} from '../features/workspace-runtime/optimistic/descriptorBuilders';
import { openWorkspaceFileInUseDialog } from '../features/workspace-runtime/state/workspaceDialogs';
import {
  hasCollectionReferenceImpact,
  notifyCollectionReferenceImpact,
} from './collectionReferenceImpact';

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

function openFileInUseRetryDialog<TVariables>(
  error: string,
  variables: TVariables,
  retry: (variables: TVariables) => void,
): boolean {
  if (!error.includes('"type":"FileInUse"')) {
    return false;
  }

  try {
    const body = JSON.parse(error) as {
      payload?: { path?: string; processes?: string[] };
    };
    const payload = body.payload;
    if (!payload?.path || !payload.processes) {
      return false;
    }

    openWorkspaceFileInUseDialog({
      path: payload.path,
      processes: payload.processes,
      onRetry: () => retry(variables),
    });
    return true;
  } catch {
    return false;
  }
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
      if (hasCollectionReferenceImpact(result.collection_impact)) {
        await publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor('collectionsCatalog'),
          'active',
        );
        notifyCollectionReferenceImpact(result.collection_impact);
      }
    },
    onError: (error, variables) => {
      const errorMessage = String(error);
      if (openFileInUseRetryDialog(errorMessage, variables, mutation.mutate)) {
        return;
      }

      if (
        errorMessage.toLowerCase().includes('not found') ||
        errorMessage.toLowerCase().includes('os error 2')
      ) {
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
    mutationFn: (params: { path: string; gameId?: string }) =>
      commands.deleteMod(params.path, params.gameId ?? null),
    onSuccess: async (result, variables) => {
      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor([thumbnailKeys.folder(variables.path)], []),
      );
      applyRuntimeEffects(queryClient, buildPathInvalidationDescriptor(variables.path, []));
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
      if (hasCollectionReferenceImpact(result.collection_impact)) {
        await publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor('collectionsCatalog'),
          'active',
        );
        notifyCollectionReferenceImpact(result.collection_impact);
      }
    },
    onError: (error, variables) => {
      const errorMessage = String(error);
      if (openFileInUseRetryDialog(errorMessage, variables, mutation.mutate)) {
        return;
      }

      if (
        errorMessage.toLowerCase().includes('not found') ||
        errorMessage.toLowerCase().includes('os error 2')
      ) {
        void runDiskRepairRecovery(queryClient, variables.gameId ?? null);
        return;
      }

      toast.error(`Delete failed: ${errorMessage}`);
    },
  });

  return mutation;
}

export function useRestoreMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: { trashId: string; gameId?: string }) =>
      commands.restoreMod(params.trashId, params.gameId ?? null),
    onSuccess: async () => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('trashState'),
        'active',
      );
    },
    onError: (error, variables) => {
      const errorMessage = String(error);
      if (openFileInUseRetryDialog(errorMessage, variables, mutation.mutate)) {
        return;
      }

      toast.error(`Restore failed: ${errorMessage}`);
    },
  });

  return mutation;
}
