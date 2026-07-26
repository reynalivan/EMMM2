/**
 * useFolderMutations — Non-core mutation hooks for mod folders.
 *
 * Owner surface for trash, metadata, import, and advanced folder hooks.
 * Multi-selection actions live in `useBulkModMutations`.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands, sparse } from '../lib/bindings';
import i18n from '../lib/i18n';
import { toast } from '../stores/useToastStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import { detailsKeys } from '../features/preview/hooks/usePreviewData';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../features/workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryInvalidationDescriptor,
  buildRuntimeMutationDescriptor,
} from '../features/workspace-runtime/optimistic/descriptorBuilders';
import { ModInfoUpdate, TrashEntry, ConflictInfo } from '../types/mod';
import { useAppStore } from '../stores/useAppStore';
import { applyRuntimePathInvalidationMutationResult } from '../features/workspace-runtime/actions/sharedRuntimeResultMapper';
import { withWatcherSuppression } from '../features/file-watcher/watcherSuppression';
import { formatBulkFailureMessage } from './bulkToastMessages';

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
export const trashKeys = {
  all: ['trash'] as const,
  list: () => [...trashKeys.all, 'list'] as const,
};

/** Hook to fetch all trashed mods. */
export function useListTrash(enabled = true) {
  return useQuery<TrashEntry[]>({
    queryKey: trashKeys.list(),
    queryFn: () => commands.listTrash(),
    enabled,
    staleTime: 30_000,
  });
}

/** Hook to permanently delete all items in the trash. */
export function useEmptyTrash() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => commands.emptyTrash(),
    onSuccess: () => {
      void publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('trashOnly'),
        'active',
      );
    },
  });
}

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
    onSuccess: async (_data, variables) => {
      // If it was selected, clear the selection pane as well
      const appStore = useAppStore.getState();
      if (appStore.gridSelection?.has(variables.folderPath)) {
        appStore.clearGridSelection();
      }

      await applyRuntimePathInvalidationMutationResult(
        queryClient,
        [variables.folderPath],
        'workspaceCorridor',
        'active',
      );
    },
  });
}

/** Hook to delete a mod's thumbnail file. */
export function useDeleteModThumbnail() {
  const queryClient = useQueryClient();
  const requireGameId = useRequireActiveGameId();

  return useMutation({
    mutationFn: async (folderPath: string) => {
      // Rust resolves the path itself here, but deleting a thumbnail with no
      // active game still means the caller is in an invalid state.
      requireGameId();
      await commands.deleteModThumbnail(folderPath);
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

// ── Import & Organize ───────────────────────────────────────────

export type ImportStrategy = 'Raw';

/** Hook to import mods from external paths (Drag & Drop). */
export function useImportMods() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: {
      paths: string[];
      targetDir: string;
      strategy: ImportStrategy;
      dbJson?: string | null;
    }) => {
      return withWatcherSuppression({ releaseDelayMs: null }, async () => {
        return commands.importModsFromPaths(
          params.paths,
          params.targetDir,
          params.strategy,
          params.dbJson ?? null,
        );
      });
    },
    onSuccess: (result) => {
      void publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderSwitch'),
        'active',
      );
      if (result.success.length > 0) {
        toast.success(
          i18n.t('grid:bulk_toast.import_success', {
            count: result.success.length,
          }),
        );
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'import'));
      }
    },
  });
}

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
