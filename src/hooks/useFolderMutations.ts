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
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../stores/useToastStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import { folderKeys, updateFolderCache } from './useFolders';
import type {
  FolderGridResponse,
  BulkResult,
  ModInfo,
  ModInfoUpdate,
  TrashEntry,
  ConflictInfo,
  DuplicateInfo,
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
    queryFn: () => invoke<TrashEntry[]>('list_trash'),
    enabled,
    staleTime: 30_000,
  });
}

/** Hook to permanently delete all items in the trash. */
export function useEmptyTrash() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke<number>('empty_trash'),
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
      invoke<void>('set_mod_category', params),
    onSuccess: (_data, variables) => {
      // Targeted: update category in cache instead of full re-listing
      updateFolderCache(queryClient, [variables.folderPath], (f) => ({
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
      invoke<string>('update_mod_thumbnail', params),
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
      invoke<void>('toggle_mod_safe', params),
    onSuccess: (_data, variables) => {
      // Targeted: update safe flag in cache instead of full re-listing
      updateFolderCache(queryClient, [variables.folderPath], (f) => ({
        ...f,
        is_safe: variables.safe,
      }));
      queryClient.invalidateQueries({ queryKey: ['objects'] });
    },
  });
}

/** Hook to delete a mod's thumbnail file. */
export function useDeleteModThumbnail() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (folderPath: string) => invoke<void>('delete_mod_thumbnail', { folderPath }),
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
      invoke<string>('paste_thumbnail', params),
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
      invoke<ModInfo>('update_mod_info', params),
    onSuccess: (_data, variables) => {
      // Targeted: update the specific folder in cache
      updateFolderCache(queryClient, [variables.folderPath], (f) => ({
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
      const match = data.children.find((f) => f.path === p);
      if (match) return match.name;
    }
    return name;
  });

  return count <= 4
    ? `${action} ${displayNames.join(', ')}`
    : `${action} ${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;
}

/** Hook to bulk toggle mods. */
export function useBulkToggle() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { paths: string[]; enable: boolean }) =>
      invoke<BulkResult>('bulk_toggle_mods', params),
    onSuccess: (result, variables) => {
      result.success.forEach((path) => {
        queryClient.removeQueries({ queryKey: thumbnailKeys.folder(path) });
      });
      // Opt-AA: Revert to full active refetch. Bulk operations alter
      // physical directory paths. Trying to accurately map all new paths
      // inside the frontend cache is a massive architectural risk that leads
      // directly to Path Instability and silent UI failures.
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });

      if (result.success.length > 0) {
        const action = variables.enable ? 'Enabled' : 'Disabled';
        toast.success(getBulkToastMessage(queryClient, result.success, action));
      }
      if (result.failures.length > 0) {
        toast.error(`Failed to toggle ${result.failures.length} items`);
      }
    },
  });
}

/** Hook to bulk delete mods. */
export function useBulkDelete() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { paths: string[]; gameId?: string }) =>
      invoke<BulkResult>('bulk_delete_mods', params),
    onSuccess: (result) => {
      result.success.forEach((path) => {
        queryClient.removeQueries({ queryKey: thumbnailKeys.folder(path) });
      });
      // Targeted cache update instead of full refetch: remove deleted folders
      updateFolderCache(queryClient, result.success, undefined, true);
      queryClient.invalidateQueries({ queryKey: ['objects'] });

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
    mutationFn: (params: { paths: string[]; update: ModInfoUpdate }) =>
      invoke<BulkResult>('bulk_update_info', params),
    onSuccess: (result, variables) => {
      // Targeted cache update instead of full refetch
      updateFolderCache(queryClient, result.success, (f) => {
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
      invoke<BulkResult>('bulk_toggle_favorite', params),
    onSuccess: (result, variables) => {
      updateFolderCache(queryClient, result.success, (f) => ({
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
      invoke<BulkResult>('bulk_pin_mods', params),
    onSuccess: (result, variables) => {
      updateFolderCache(queryClient, result.success, (f) => ({
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
      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        return await invoke<BulkResult>('import_mods_from_paths', params);
      } finally {
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    },
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
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
      invoke<BulkResult>('auto_organize_mods', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
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
    mutationFn: (params: { targetPath: string; gameId: string }) =>
      invoke<BulkResult>('enable_only_this', params),
    onSuccess: (result) => {
      result.success.forEach((path) => {
        queryClient.removeQueries({ queryKey: thumbnailKeys.folder(path) });
      });
      // Opt-Z3: Revert to full invalidation. This touches potentially hundreds of
      // folders physically, and guessing all their new paths in cache is a massive
      // desync risk. It is a rare operation where an active refetch is 100% justified.
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });

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
      invoke<DuplicateInfo[]>('check_duplicate_enabled', params),
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
        ? invoke<ConflictInfo[]>('get_active_mod_conflicts', { gameId: activeGame.id })
        : Promise.resolve([]),
    enabled: !!activeGame?.id,
    staleTime: 60_000, // Conflicts rarely change — watcher invalidates on toggle
  });
}
