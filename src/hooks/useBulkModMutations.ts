/**
 * useBulkModMutations — Multi-selection mutation hooks for mod folders.
 *
 * Owner surface for the bulk grid actions (toggle, delete, info, favorite, pin).
 */

import { useMutation, useQueryClient, QueryClient } from '@tanstack/react-query';
import { commands, sparse } from '../lib/bindings';
import { toast } from '../stores/useToastStore';
import { thumbnailKeys } from './useThumbnail';
import { folderKeys } from './folderCache';
import { stripDisabledPrefix } from '../lib/disabledPrefix';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../features/workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../features/workspace-runtime/optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../features/workspace-runtime/optimistic/descriptor';
import { FolderGridResponse, ModInfoUpdate, ModFolder } from '../types/mod';
import { openWorkspaceFileInUseDialog } from '../features/workspace-runtime/state/workspaceDialogs';
import { extractFileInUsePayload, formatAppError } from '../lib/appError';
import {
  hasCollectionReferenceImpact,
  notifyCollectionReferenceImpact,
} from './collectionReferenceImpact';
import {
  formatBulkFailureMessage,
  formatBulkSuccessMessage,
  type BulkSuccessActionKey,
} from './bulkToastMessages';
import { resolveTogglePathRewrites } from './folderMutationPayloads';

/** Prefers the name already cached for a path, falling back to its folder name. */
function getBulkToastMessage(
  queryClient: QueryClient,
  paths: string[],
  actionKey: BulkSuccessActionKey,
): string {
  if (paths.length === 0) return '';

  const displayNames = paths.map((p) => {
    const name = stripDisabledPrefix(p.split(/[/\\]/).pop() || '');

    const prevQueries = queryClient.getQueriesData<FolderGridResponse>({
      queryKey: folderKeys.all,
    });
    for (const [, data] of prevQueries) {
      if (!data) continue;
      const match = data.children.find((f: ModFolder) => f.path === p);
      if (match) return match.name;
    }
    return name;
  });

  return formatBulkSuccessMessage(displayNames, actionKey);
}

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
      applyRuntimeEffects(
        queryClient,
        mergeRuntimeEffectDescriptors(
          buildQueryRemovalDescriptor(
            result.success.map((newPath) => thumbnailKeys.folder(newPath)),
            [],
          ),
          buildWorkspacePathRewritesDescriptor(pathRewrites, []),
        ),
      );
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderSwitch'),
        'active',
      );

      if (result.success.length > 0) {
        const action = variables.enable ? 'enabled' : 'disabled';
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (hasCollectionReferenceImpact(result.collection_impact)) {
        await publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor('collectionsCatalog'),
          'active',
        );
        notifyCollectionReferenceImpact(result.collection_impact);
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'toggle'));
      }
    },
    onError: (error, variables) => {
      const payload = extractFileInUsePayload(error);
      if (payload) {
        openWorkspaceFileInUseDialog({
          path: payload.path,
          processes: payload.processes,
          onRetry: () => mutation.mutate(variables),
        });
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
    mutationFn: (params: { paths: string[]; gameId?: string }) =>
      commands.bulkDeleteMods(params.gameId ?? null, params.paths),
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
        buildRuntimeMutationDescriptor([
          'workspaceStructure',
          'workspaceCorridor',
          'dashboardKeybindings',
        ]),
        'active',
      );

      if (result.success.length > 0) {
        toast.success(getBulkToastMessage(queryClient, result.success, 'deleted'));
      }
      if (hasCollectionReferenceImpact(result.collection_impact)) {
        await publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor('collectionsCatalog'),
          'active',
        );
        notifyCollectionReferenceImpact(result.collection_impact);
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'delete'));
      }
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
        toast.success(getBulkToastMessage(queryClient, result.success, 'updated'));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'update'));
      }
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
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'favorite'));
      }
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
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'pin'));
      }
    },
  });
}
