/**
 * useBulkModMutations — Multi-selection mutation hooks for mod folders.
 *
 * Bulk toggle, delete and favorite actions for explicit folder paths.
 */

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import { thumbnailKeys } from '@/entities/mod';
import {
  cancelRuntimeDescriptorQueries,
  publishRuntimeDescriptor,
} from '@/shared/lib/queryRefresh';
import { applyRuntimeEffects } from '@/features/workspace-runtime/@x/mod-runtime';
import {
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '@/features/workspace-runtime/@x/mod-runtime';
import { formatAppError } from '../../../shared/lib/appError';
import {
  openBulkFileLockErrorDialog,
  openBulkFileInUseRetryDialog,
} from '@/features/workspace-runtime/@x/mod-runtime';
import { nextWorkspaceIntentRevision } from '@/features/workspace-runtime/@x/mod-runtime';
import {
  collectionReferenceImpactRefreshEvents,
  notifyCollectionReferenceImpact,
} from '@/features/workspace-runtime/@x/mod-runtime';
import {
  formatBulkCancelledMessage,
  formatBulkFailureMessage,
  formatBulkSuccessMessage,
} from '../../../shared/lib/hooks/bulkToastMessages';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';
import { useAppStore } from '@/app/store';

function createBulkOperationId(kind: 'toggle' | 'delete'): string {
  return `${kind}-${crypto.randomUUID()}`;
}

function isActiveGame(gameId: string): boolean {
  return useAppStore.getState().activeGameId === gameId;
}

/** Hook to bulk toggle mods. */
export function useBulkToggle() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    // Bulk toggle is an explicit runtime switch path.
    // Global runtime refresh comes from one final publish, not per-item ad-hoc invalidation.
    mutationFn: (params: {
      gameId: string;
      paths: string[];
      enable: boolean;
      recoveryIds?: string[];
    }) =>
      commands.bulkToggleMods(
        params.gameId,
        params.paths,
        params.enable,
        createBulkOperationId('toggle'),
        nextWorkspaceIntentRevision(),
        null,
        params.recoveryIds ?? null,
      ),

    onSuccess: (result, variables) => {
      const currentGame = isActiveGame(variables.gameId);
      if (currentGame) {
        const descriptor = buildRuntimeMutationDescriptor(
          'folderSwitch',
          collectionReferenceImpactRefreshEvents(result.collection_impact),
        );
        const cancellation = cancelRuntimeDescriptorQueries(queryClient, descriptor);
        // No thumbnail drop: a toggle keeps the folder's identity, and the
        // cache is identity-keyed — dropping here would evict the entry the
        // new path is about to reuse and force a regeneration per mod.
        applyRuntimeEffects(
          queryClient,
          buildWorkspacePathRewritesDescriptor(result.path_rewrites, []),
        );
        void cancellation
          .then(() => {
            if (!isActiveGame(variables.gameId)) {
              return;
            }
            return publishRuntimeDescriptor(queryClient, descriptor, 'active');
          })
          .catch((error: unknown) => {
            console.error('[BulkToggle] Background cache refresh failed:', error);
          });
      }

      if (!currentGame) {
        return;
      }
      if (result.success.length > 0) {
        const action = variables.enable ? 'enabled' : 'disabled';
        toast.success(formatBulkSuccessMessage(result.success, action));
      }
      if (result.collection_impact) notifyCollectionReferenceImpact(result.collection_impact);
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'toggle'));
        openBulkFileInUseRetryDialog(result, variables, mutation.mutateAsync);
      }
      if (result.cancelled) {
        toast.info(formatBulkCancelledMessage(result));
      }
      notifyCommittedMutationSyncWarning(result);
    },
    onError: (error, variables) => {
      if (!isActiveGame(variables.gameId)) {
        return;
      }
      if (openBulkFileLockErrorDialog(error, variables, mutation.mutateAsync)) {
        return;
      }
      toast.error(formatAppError(error));
    },
  });

  return mutation;
}

/** Hook to bulk delete mods. */
export function useBulkDelete() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    // `gameId` names both the mods root the paths must sit inside and the
    // game whose index rows get pruned; the backend refuses without it.
    mutationFn: (params: { paths: string[]; gameId: string; recoveryIds?: string[] }) =>
      commands.bulkDeleteMods(
        params.gameId,
        params.paths,
        createBulkOperationId('delete'),
        params.recoveryIds ?? null,
      ),
    onSuccess: async (result, variables) => {
      if (!isActiveGame(variables.gameId)) {
        return;
      }
      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor(
          result.success.map((path) => thumbnailKeys.folder(path)),
          [],
        ),
      );
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor(
          ['workspaceStructure', 'workspaceRuntime', 'dashboardKeybindings'],
          collectionReferenceImpactRefreshEvents(result.collection_impact),
        ),
        'active',
      );
      if (!isActiveGame(variables.gameId)) {
        return;
      }
      if (result.success.length > 0) {
        toast.success(formatBulkSuccessMessage(result.success, 'deleted'));
      }
      if (result.collection_impact) notifyCollectionReferenceImpact(result.collection_impact);
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'delete'));
        openBulkFileInUseRetryDialog(result, variables, mutation.mutateAsync);
      }
      if (result.cancelled) {
        toast.info(formatBulkCancelledMessage(result));
      }
      notifyCommittedMutationSyncWarning(result);
    },
    onError: (error, variables) => {
      if (!isActiveGame(variables.gameId)) return;
      if (openBulkFileLockErrorDialog(error, variables, mutation.mutateAsync)) return;
      toast.error(formatAppError(error));
    },
  });
  return mutation;
}

/** Hook to bulk toggle favorite with targeted cache update. */
export function useBulkFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPaths: string[]; favorite: boolean }) =>
      commands.bulkToggleFavorite(params.gameId, params.folderPaths, params.favorite),
    onSuccess: async (result, variables) => {
      if (!isActiveGame(variables.gameId)) {
        return;
      }
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderMetadataPreview'),
        'active',
      );
      if (!isActiveGame(variables.gameId)) {
        return;
      }
      if (result.success.length > 0) {
        const action = variables.favorite ? 'favorited' : 'unfavorited';
        toast.success(formatBulkSuccessMessage(result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'favorite'));
      }
      notifyCommittedMutationSyncWarning(result);
    },
  });
}
