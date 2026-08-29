/**
 * useBulkModMutations — Multi-selection mutation hooks for mod folders.
 *
 * Owner surface for the bulk grid actions (toggle, delete, info, favorite, pin).
 */

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { commands, sparse } from '../../../core/tauri/bindings';
import { toast } from '../../../stores/useToastStore';
import { thumbnailKeys } from '../../dashboard/hooks/useThumbnail';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../../workspace-runtime/optimistic/descriptorBuilders';
import type { ModInfoUpdate } from '../../../types/object';
import { formatAppError } from '../../../core/lib/appError';
import { openFileInUseRetryDialog } from '../../../shared/hooks/fileInUseRetry';
import {
  collectionReferenceImpactRefreshEvents,
  notifyCollectionReferenceImpact,
} from '../../collections/hooks/collectionReferenceImpact';
import { formatBulkFailureMessage, formatBulkSuccessMessage } from '../../../shared/hooks/bulkToastMessages';
import { resolveTogglePathRewrites } from '../../folder-grid/hooks/folderMutationPayloads';
import { notifyCommittedMutationSyncWarning } from '../../../core/lib/committedMutationWarning';

/** Hook to bulk toggle mods. */
export function useBulkToggle() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    // Bulk toggle is an explicit runtime switch path.
    // Global runtime refresh comes from one final publish, not per-item ad-hoc invalidation.
    mutationFn: (params: { gameId: string; paths: string[]; enable: boolean }) =>
      commands.bulkToggleMods(params.gameId, params.paths, params.enable),

    onSuccess: async (result, variables) => {
      const pathRewrites = resolveTogglePathRewrites(
        result.success,
        result.path_rewrites,
        variables.enable,
      );
      // No thumbnail drop: a toggle keeps the folder's identity, and the
      // cache is identity-keyed — dropping here would evict the entry the
      // new path is about to reuse and force a regeneration per mod.
      applyRuntimeEffects(queryClient, buildWorkspacePathRewritesDescriptor(pathRewrites, []));
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor(
          'folderSwitch',
          collectionReferenceImpactRefreshEvents(result.collection_impact),
        ),
        'active',
      );

      if (result.success.length > 0) {
        const action = variables.enable ? 'enabled' : 'disabled';
        toast.success(formatBulkSuccessMessage(result.success, action));
      }
      if (result.collection_impact) notifyCollectionReferenceImpact(result.collection_impact);
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'toggle'));
      }
      notifyCommittedMutationSyncWarning(result);
    },
    onError: (error, variables) => {
      if (openFileInUseRetryDialog(error, variables, mutation.mutate)) {
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

  return useMutation({
    // `gameId` names both the mods root the paths must sit inside and the
    // game whose index rows get pruned; the backend refuses without it.
    mutationFn: (params: { paths: string[]; gameId: string }) =>
      commands.bulkDeleteMods(params.gameId, params.paths),
    onSuccess: async (result) => {
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

      if (result.success.length > 0) {
        toast.success(formatBulkSuccessMessage(result.success, 'deleted'));
      }
      if (result.collection_impact) notifyCollectionReferenceImpact(result.collection_impact);
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'delete'));
      }
      notifyCommittedMutationSyncWarning(result);
    },
  });
}

/** Hook to bulk update info.json. */
export function useBulkUpdateInfo() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; paths: string[]; update: ModInfoUpdate }) =>
      commands.bulkUpdateInfo(params.gameId, params.paths, sparse(params.update)),
    onSuccess: async (result) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderMetadataPreview'),
        'active',
      );
      if (result.success.length > 0) {
        toast.success(formatBulkSuccessMessage(result.success, 'updated'));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'update'));
      }
      notifyCommittedMutationSyncWarning(result);
    },
  });
}

/** Classify terminal mods selected directly or through parent object folders. */
export function useBulkSafety() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; paths: string[]; safe: boolean }) =>
      commands.bulkSetModSafety(params.gameId, params.paths, params.safe),
    onSuccess: async (result, variables) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('safetyClassification'),
        'active',
      );
      if (result.success.length > 0) {
        toast.success(
          formatBulkSuccessMessage(
            result.success,
            variables.safe ? 'marked_safe' : 'marked_unsafe',
          ),
        );
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'safety'));
      }
      notifyCommittedMutationSyncWarning(result);
    },
  });
}

/** Hook to bulk toggle favorite with targeted cache update. */
export function useBulkFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPaths: string[]; favorite: boolean }) =>
      commands.bulkToggleFavorite(params.gameId, params.folderPaths, params.favorite),
    onSuccess: async (result, variables) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderMetadataPreview'),
        'active',
      );
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

/** Hook to bulk pin/unpin mods with targeted cache update. */
export function useBulkPin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPaths: string[]; pin: boolean }) =>
      commands.bulkPinMods(params.gameId, params.folderPaths, params.pin),
    onSuccess: async (result, variables) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderMetadataPreview'),
        'active',
      );
      if (result.success.length > 0) {
        const action = variables.pin ? 'pinned' : 'unpinned';
        toast.success(formatBulkSuccessMessage(result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'pin'));
      }
      notifyCommittedMutationSyncWarning(result);
    },
  });
}
