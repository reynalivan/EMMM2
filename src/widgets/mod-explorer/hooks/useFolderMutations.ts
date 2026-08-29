/**
 * useFolderMutations — Non-core mutation hooks for mod folders.
 *
 * Owner surface for trash, metadata, import, and advanced folder hooks.
 * Multi-selection actions live in `useBulkModMutations`.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands, sparse } from '../../../shared/api/tauri/bindings';
import { useActiveGame } from '@/pages/dashboard/hooks/useActiveGame';
import { thumbnailKeys } from '@/pages/dashboard/hooks/useThumbnail';
import { detailsKeys } from '@/widgets/mod-preview/hooks/usePreviewData';
import { publishRuntimeDescriptor } from '@/features/runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '@/features/workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryInvalidationDescriptor,
  buildRuntimeMutationDescriptor,
} from '@/features/workspace-runtime/optimistic/descriptorBuilders';
import { ModInfoUpdate } from '@/entities/game-object/model/object';
import { ConflictInfo } from '@/entities/workspace/model/scanner';
import { useAppStore } from '../../../app/store/useAppStore';
import { applyRuntimePathInvalidationMutationResult } from '@/features/workspace-runtime/actions/sharedRuntimeResultMapper';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';

/**
 * Getter for the active game id that throws when there is none.
 *
 * Every mod mutation below is scoped to the active game on the Rust side, and
 * the check has to run when the mutation fires rather than at render time — so
 * this hands back a getter instead of the id itself.
 */
function useRequireActiveGameId(): () => string {
  const { activeGame } = useActiveGame();

  return () => {
    if (!activeGame?.id) {
      throw new Error('No active game selected');
    }

    return activeGame.id;
  };
}

// ── Trash ───────────────────────────────────────────────────────

/** Query key for trash listing. */
// ── Metadata Mutations ──────────────────────────────────────────

/** Hook to update a mod's category (object type). */
export function useUpdateModCategory() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPath: string; category: string }) =>
      commands.setModCategory(params.gameId, params.folderPath, params.category),
    onSuccess: () => {
      void publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderMetadataPreview'),
        'active',
      );
    },
  });
}

/** Hook to update a mod's thumbnail. */
export function useUpdateModThumbnail() {
  const queryClient = useQueryClient();
  const requireGameId = useRequireActiveGameId();

  return useMutation({
    mutationFn: (params: { folderPath: string; sourcePath: string }) =>
      commands.updateModThumbnail(requireGameId(), params.folderPath, params.sourcePath),
    onSuccess: async (_data, variables) => {
      const descriptor = buildQueryInvalidationDescriptor(
        [thumbnailKeys.folder(variables.folderPath)],
        [],
      );
      applyRuntimeEffects(queryClient, descriptor);
    },
  });
}

/** Hook to toggle a mod's safe classification. */
export function useToggleModSafe() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPath: string; safe: boolean }) =>
      commands.toggleModSafe(params.gameId, params.folderPath, params.safe),
    onSuccess: async (result, variables) => {
      // If it was selected, clear the selection pane as well
      const appStore = useAppStore.getState();
      if (appStore.gridSelection?.has(variables.folderPath)) {
        appStore.clearGridSelection();
      }

      await applyRuntimePathInvalidationMutationResult(
        queryClient,
        [variables.folderPath],
        'safetyClassification',
        'active',
      );
      notifyCommittedMutationSyncWarning(result);
    },
  });
}

/** Hook to delete a mod's thumbnail file. */
export function useDeleteModThumbnail() {
  const queryClient = useQueryClient();
  const requireGameId = useRequireActiveGameId();

  return useMutation({
    mutationFn: async (folderPath: string) => {
      await commands.deleteModThumbnail(requireGameId(), folderPath);
    },
    onSuccess: async (_data, folderPath) => {
      const descriptor = buildQueryInvalidationDescriptor(
        [thumbnailKeys.folder(folderPath), detailsKeys.previewImages(folderPath)],
        [],
      );
      applyRuntimeEffects(queryClient, descriptor);
    },
  });
}

/** Hook to paste a thumbnail from clipboard bytes. */
export function usePasteThumbnail() {
  const queryClient = useQueryClient();
  const requireGameId = useRequireActiveGameId();

  return useMutation({
    mutationFn: (params: { folderPath: string; imageData: number[] }) =>
      commands.pasteThumbnail(requireGameId(), params.folderPath, params.imageData),
    onSuccess: async (_data, variables) => {
      const descriptor = buildQueryInvalidationDescriptor(
        [thumbnailKeys.folder(variables.folderPath)],
        [],
      );
      applyRuntimeEffects(queryClient, descriptor);
    },
  });
}

// ── Single-Item Info ────────────────────────────────────────────

export function useUpdateModInfo() {
  const queryClient = useQueryClient();
  const requireGameId = useRequireActiveGameId();

  return useMutation({
    mutationFn: (params: { folderPath: string; update: ModInfoUpdate }) =>
      commands.updateModInfo(requireGameId(), params.folderPath, sparse(params.update)),
    onSuccess: async () => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderMetadataPreview'),
        'active',
      );
    },
  });
}

// ── Conflict queries ─────────────────────────────────────────────

/**
 * Hook to get all active conflicts for the current game.
 * Covers: US-5.7
 */
export function useActiveConflicts() {
  const { activeGame } = useActiveGame();

  return useQuery<ConflictInfo[]>({
    queryKey: ['conflicts', activeGame?.id],
    queryFn: () =>
      activeGame?.id ? commands.getActiveModConflicts(activeGame.id) : Promise.resolve([]),
    enabled: !!activeGame?.id,
    staleTime: 60_000, // Conflicts rarely change — watcher invalidates on toggle
  });
}
