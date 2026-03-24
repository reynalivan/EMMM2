/**
 * useFolderMutations — Non-core mutation hooks for mod folders.
 *
 * Extracted from useFolders.ts to keep that file under 350 lines.
 * Contains: trash, metadata, bulk, import, and advanced operation hooks.
 *
 * All hooks are barrel-re-exported from useFolders.ts so consumers
 * don't need to change their import paths.
 */

import { useQuery, useMutation, useQueryClient, QueryClient } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import { toast } from '../stores/useToastStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import { folderKeys, updateFolderCache } from './useFolders';
import { corridorKeys } from '../features/collections/queryKeys';
import {
  FolderGridResponse,
  ModInfoUpdate,
  TrashEntry,
  ConflictInfo,
  ModFolder,
} from '../types/mod';

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
      queryClient.invalidateQueries({ queryKey: trashKeys.all });
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
      queryClient.invalidateQueries({ queryKey: ['objects'] });
    },
  });
}

/** Hook to update a mod's thumbnail. */
export function useUpdateModThumbnail() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { folderPath: string; sourcePath: string }) =>
      commands.updateModThumbnail(params),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.folder(variables.folderPath) });
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

      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
    },
  });
}

/** Hook to delete a mod's thumbnail file. */
export function useDeleteModThumbnail() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (folderPath: string) => commands.deleteModThumbnail({ folderPath }),
    onSuccess: (_data, folderPath) => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.folder(folderPath) });
    },
  });
}

/** Hook to paste a thumbnail from clipboard bytes. */
export function usePasteThumbnail() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { folderPath: string; imageData: number[] }) =>
      commands.pasteThumbnail(params),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.folder(variables.folderPath) });
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

export type ImportStrategy = 'Raw' | 'AutoOrganize';

/** Helper to construct bulk toast messages with explicit names. */
function getBulkToastMessage(queryClient: QueryClient, paths: string[], action: string): string {
  const count = paths.length;
  if (count === 0) return '';

  const displayNames = paths.map((p) => {
    let name = p.split(/[/\\]/).pop() || '';
    if (name.startsWith('DISABLED ')) name = name.substring(9);

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

import { useAppStore } from '../stores/useAppStore';

/** Hook to bulk toggle mods. */
export function useBulkToggle() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: { gameId: string; paths: string[]; enable: boolean }) =>
      commands.bulkToggleMods(params),

    onSuccess: async (result, variables) => {
      result.success.forEach((newPath) => {
        queryClient.removeQueries({ queryKey: thumbnailKeys.folder(newPath) });

        // Derive old path to keep grid selection alive
        const namePart = newPath.split(/[/\\]/).pop() || '';
        const guessedOldName = variables.enable
          ? `DISABLED ${namePart}`
          : namePart.replace(/^DISABLED /, '');
        const oldPath = newPath.slice(0, -namePart.length) + guessedOldName;
        useAppStore.getState().replaceGridSelection(oldPath, newPath);
      });
      // Opt-AA: Revert to full active refetch. Bulk operations alter
      // physical directory paths. Trying to accurately map all new paths
      // inside the frontend cache is a massive architectural risk that leads
      // directly to Path Instability and silent UI failures.
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      const activeGameId = useAppStore.getState().activeGameId;
      if (activeGameId) {
        queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      }

      if (result.success.length > 0) {
        const action = variables.enable ? 'Enabled' : 'Disabled';
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(`Failed to toggle ${result.failures.length} items`);
      }
    },
    onError: (error, variables) => {
      const errStr = String(error);
      if (errStr.includes('"type":"FileInUse"')) {
        try {
          const body = JSON.parse(errStr);
          const payload = body.payload;
          useAppStore
            .getState()
            .openFileInUseDialog(payload.path, payload.processes, () =>
              mutation.mutate(variables),
            );
          return;
        } catch {
          /* parse failed */
        }
      }
      toast.error(errStr);
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
      result.success.forEach((path) => {
        queryClient.removeQueries({ queryKey: thumbnailKeys.folder(path) });
      });
      // Targeted cache update instead of full refetch: remove deleted folders
      updateFolderCache(queryClient, result.success, undefined, true);
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });

      if (result.success.length > 0) {
        toast.success(getBulkToastMessage(queryClient, result.success, 'Deleted'));
      }
      if (result.failures.length > 0) {
        toast.error(`Failed to delete ${result.failures.length} items`);
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
        toast.error(`Failed to update ${result.failures.length} items`);
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
        toast.error(`Failed to update ${result.failures.length} items`);
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
        toast.error(`Failed to update ${result.failures.length} items`);
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
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      if (result.success.length > 0) {
        toast.success(`Imported ${result.success.length} items`);
      }
      if (result.failures.length > 0) {
        toast.error(`Failed to import ${result.failures.length} items`);
      }
    },
  });
}

/** Hook to auto-organize existing mods. */
export function useAutoOrganizeMods() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { paths: string[]; targetRoot: string; dbJson: string }) =>
      commands.autoOrganizeMods(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
    },
  });
}

// ── Advanced Mod Operations ─────────────────────────────────────

/**
 * Enable a single mod and disable all others sharing the same object.
 * Covers: TC-5.3-01 (Enable Only This)
 */
export function useEnableOnlyThis() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { targetPath: string; gameId: string }) => commands.enableOnlyThis(params),
    onSuccess: (result) => {
      // The first item is the one enabled, the rest are disabled
      result.success.forEach((newPath: string, idx: number) => {
        queryClient.removeQueries({ queryKey: thumbnailKeys.folder(newPath) });

        // Derive old path to keep grid selection alive
        const isEnabled = idx === 0;
        const namePart = newPath.split(/[/\\]/).pop() || '';
        const guessedOldName = isEnabled
          ? `DISABLED ${namePart}`
          : namePart.replace(/^DISABLED /, '');
        const oldPath = newPath.slice(0, -namePart.length) + guessedOldName;
        useAppStore.getState().replaceGridSelection(oldPath, newPath);
      });
      // Opt-Z3: Revert to full invalidation. This touches potentially hundreds of
      // folders physically, and guessing all their new paths in cache is a massive
      // desync risk. It is a rare operation where an active refetch is 100% justified.
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });

      const disabled = result.success.length - 1;
      if (disabled > 0) {
        toast.info(`Disabled ${disabled} other mod(s)`);
      }
    },
    onError: (error) => {
      toast.error(String(error));
    },
  });
}

/**
 * Check for duplicate enabled mods before toggling.
 * Covers: NC-5.2-03 (Duplicate Character Warning)
 */
export function useCheckDuplicate() {
  return useMutation({
    mutationFn: (params: { folderPath: string; gameId: string }) =>
      commands.checkDuplicateEnabled(params),
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
