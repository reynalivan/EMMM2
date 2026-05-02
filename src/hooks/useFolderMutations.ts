/**
 * useFolderMutations — Non-core mutation hooks for mod folders.
 *
 * Owner surface for trash, metadata, bulk, import, and advanced folder hooks.
 */

import { useQuery, useMutation, useQueryClient, QueryClient } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import { toast } from '../stores/useToastStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import { folderKeys, updateFolderCache } from './folderCache';
import { stripDisabledPrefix, toggleDisabledInPath } from '../lib/disabledPrefix';
import { detailsKeys } from '../features/preview/hooks/usePreviewData';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import { applyRuntimeEffects } from '../features/workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryInvalidationDescriptor,
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
} from '../features/workspace-runtime/optimistic/descriptorBuilders';
import {
  FolderGridResponse,
  ModInfoUpdate,
  TrashEntry,
  ConflictInfo,
  ModFolder,
} from '../types/mod';
import { useAppStore } from '../stores/useAppStore';
import { openWorkspaceFileInUseDialog } from '../features/workspace-runtime/state/workspaceDialogs';
import { extractFileInUsePayload, formatAppError } from '../lib/appError';
import { applyRuntimePathInvalidationMutationResult } from '../features/workspace-runtime/actions/sharedRuntimeResultMapper';

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
      commands.setModCategory(params),
    onSuccess: (_data, variables) => {
      // Targeted: update category in cache instead of full re-listing
      updateFolderCache(queryClient, [variables.folderPath], (f: ModFolder) => ({
        ...f,
        category: variables.category,
      }));
      void publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('workspaceOnly'),
        'active',
      );
    },
  });
}

/** Hook to update a mod's thumbnail. */
export function useUpdateModThumbnail() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { folderPath: string; sourcePath: string }) =>
      commands.updateModThumbnail(params),
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
      commands.toggleModSafe(params),
    onSuccess: async (_data, variables) => {
      // Phase 24 barrier: The mod just switched contexts.
      // Remove it from the current grid view aggressively so it doesn't linger.
      updateFolderCache(queryClient, [variables.folderPath], undefined, true);

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
  const { activeGame } = useActiveGame();

  return useMutation({
    mutationFn: async (folderPath: string) => {
      if (!activeGame?.id) {
        throw new Error('No active game selected');
      }

      await commands.deleteModThumbnail({ folderPath });
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

  return useMutation({
    mutationFn: (params: { folderPath: string; imageData: number[] }) =>
      commands.pasteThumbnail(params),
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

  return useMutation({
    mutationFn: (params: { folderPath: string; update: ModInfoUpdate }) =>
      commands.updateModInfo(params),
    onSuccess: (_data, variables) => {
      // Targeted: update the specific folder in cache
      updateFolderCache(queryClient, [variables.folderPath], (f: ModFolder) => ({
        ...f,
        metadata: variables.update.metadata
          ? { ...f.metadata, ...variables.update.metadata }
          : f.metadata,
        is_favorite: variables.update.is_favorite ?? f.is_favorite,
        is_safe: variables.update.is_safe ?? f.is_safe,
      }));
    },
  });
}

// ── Bulk Operations ─────────────────────────────────────────────

export type ImportStrategy = 'Raw';

/** Helper to construct bulk toast messages with explicit names. */
function getBulkToastMessage(queryClient: QueryClient, paths: string[], action: string): string {
  const count = paths.length;
  if (count === 0) return '';

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

  return count <= 4
    ? `${action} ${displayNames.join(', ')}`
    : `${action} ${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;
}

function formatBulkFailureMessage(
  failures: { path: string; error: unknown }[],
  action: string,
): string {
  if (failures.length === 0) {
    return '';
  }

  const firstFailure = failures[0];
  const firstName = stripDisabledPrefix(
    firstFailure.path.split(/[/\\]/).pop() || firstFailure.path,
  );
  const reason = formatAppError(firstFailure.error);
  if (failures.length === 1) {
    return `${action} failed for ${firstName}: ${reason}`;
  }

  return `${action} failed for ${firstName} + ${failures.length - 1} others: ${reason}`;
}

/** Hook to bulk toggle mods. */
export function useBulkToggle() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    // Bulk toggle is an explicit runtime switch path.
    // Global runtime refresh comes from one final publish, not per-item ad-hoc invalidation.
    mutationFn: (params: { gameId: string; paths: string[]; enable: boolean }) =>
      commands.bulkToggleMods(params),

    onSuccess: async (result, variables) => {
      const removalDescriptor = buildQueryRemovalDescriptor(
        result.success.map((newPath) => thumbnailKeys.folder(newPath)),
        [],
      );
      applyRuntimeEffects(queryClient, removalDescriptor);
      result.success.forEach((newPath) => {
        // Derive old path to keep grid selection alive
        const oldPath = toggleDisabledInPath(newPath, !variables.enable);
        useAppStore.getState().replaceGridSelection(oldPath, newPath);
      });
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderSwitch'),
        'active',
      );

      if (result.success.length > 0) {
        const action = variables.enable ? 'Enabled' : 'Disabled';
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'Toggle'));
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
    mutationFn: (params: { paths: string[]; gameId?: string }) => commands.bulkDeleteMods(params),
    onSuccess: async (result) => {
      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor(
          result.success.map((path) => thumbnailKeys.folder(path)),
          [],
        ),
      );
      // Targeted cache update instead of full refetch: remove deleted folders
      updateFolderCache(queryClient, result.success, undefined, true);
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor(['workspaceCorridor', 'dashboardKeybindings']),
        'active',
      );

      if (result.success.length > 0) {
        toast.success(getBulkToastMessage(queryClient, result.success, 'Deleted'));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'Delete'));
      }
    },
  });
}

/** Hook to bulk update info.json. */
export function useBulkUpdateInfo() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; paths: string[]; update: ModInfoUpdate }) =>
      commands.bulkUpdateInfo(params),
    onSuccess: (result, variables) => {
      // Targeted cache update instead of full refetch
      updateFolderCache(queryClient, result.success, (f: ModFolder) => {
        const update = variables.update;
        return {
          ...f,
          is_favorite: update.is_favorite ?? f.is_favorite,
          is_safe: update.is_safe ?? f.is_safe,
          metadata: update.metadata ? { ...f.metadata, ...update.metadata } : f.metadata,
        };
      });
      if (result.success.length > 0) {
        toast.success(getBulkToastMessage(queryClient, result.success, 'Updated'));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'Update'));
      }
    },
  });
}

/** Hook to bulk toggle favorite with targeted cache update. */
export function useBulkFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPaths: string[]; favorite: boolean }) =>
      commands.bulkToggleFavorite(params),
    onSuccess: (result, variables) => {
      updateFolderCache(queryClient, result.success, (f: ModFolder) => ({
        ...f,
        is_favorite: variables.favorite,
      }));
      if (result.success.length > 0) {
        const action = variables.favorite ? 'Favorited' : 'Unfavorited';
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'Favorite'));
      }
    },
  });
}

/** Hook to bulk pin/unpin mods with targeted cache update. */
export function useBulkPin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPaths: string[]; pin: boolean }) =>
      commands.bulkPinMods(params),
    onSuccess: (result, variables) => {
      updateFolderCache(queryClient, result.success, (f: ModFolder) => ({
        ...f,
        is_pinned: variables.pin,
      }));
      if (result.success.length > 0) {
        const action = variables.pin ? 'Pinned' : 'Unpinned';
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'Pin'));
      }
    },
  });
}

// ── Import & Organize ───────────────────────────────────────────

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
      await commands.setWatcherSuppression({ suppressed: true });
      try {
        return await commands.importModsFromPaths({
          ...params,
          dbJson: params.dbJson ?? undefined,
        });
      } finally {
        await commands.setWatcherSuppression({ suppressed: false });
      }
    },
    onSuccess: (result) => {
      void publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('folderSwitch'),
        'active',
      );
      if (result.success.length > 0) {
        toast.success(`Imported ${result.success.length} items`);
      }
      if (result.failures.length > 0) {
        toast.error(formatBulkFailureMessage(result.failures, 'Import'));
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
      activeGame?.id
        ? commands.getActiveModConflicts({ gameId: activeGame.id })
        : Promise.resolve([]),
    enabled: !!activeGame?.id,
    staleTime: 60_000, // Conflicts rarely change — watcher invalidates on toggle
  });
}
