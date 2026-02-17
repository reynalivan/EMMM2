/**
 * Hook for fetching raw mod folder listings from the filesystem.
 * Used as the sidebar/explorer data source.
 * Enhanced for Epic 4 with sub_path navigation, mutations, and sorting.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../stores/useToastStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import type {
  ModFolder,
  RenameResult,
  SortField,
  SortOrder,
  BulkResult,
  ModInfo,
  ModInfoUpdate,
  TrashEntry,
  ConflictInfo,
  DuplicateInfo,
} from '../types/mod';

// Re-export for consumers that import from here
export type { ModFolder, ModInfo };

/** Query key factory for folder cache management */
export const folderKeys = {
  all: ['mod-folders'] as const,
  list: (modsPath: string, subPath?: string, objectId?: string) =>
    [...folderKeys.all, modsPath, subPath ?? '', objectId ?? ''] as const,
};

/**
 * Fetch mod folders from the active game's mods_path directory.
 * Supports sub_path for deep navigation and objectId for DB-level filtering by object.
 */
export function useModFolders(subPath?: string, objectId?: string) {
  const { activeGame } = useActiveGame();
  const modsPath = activeGame?.mod_path;

  return useQuery<ModFolder[]>({
    queryKey: folderKeys.list(modsPath ?? '', subPath, objectId),
    queryFn: () =>
      invoke<ModFolder[]>('list_mod_folders', {
        gameId: activeGame?.id ?? null,
        modsPath: modsPath!,
        subPath: subPath || null,
        objectId: objectId || null,
      }),
    enabled: !!modsPath,
    staleTime: 10_000,
    refetchOnWindowFocus: false,
    retry: 2,
  });
}

/** Sort folders client-side by field and direction. */
export function sortFolders(folders: ModFolder[], field: SortField, order: SortOrder): ModFolder[] {
  const sorted = [...folders].sort((a, b) => {
    // Favorites always on top
    if (a.is_favorite !== b.is_favorite) {
      return a.is_favorite ? -1 : 1;
    }

    switch (field) {
      case 'name':
        return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
      case 'modified_at':
        return a.modified_at - b.modified_at;
      case 'size_bytes':
        return a.size_bytes - b.size_bytes;
      default:
        return 0;
    }
  });
  return order === 'desc' ? sorted.reverse() : sorted;
}

/**
 * Toggle a mod's enabled/disabled state with undo toast.
 * Shows an "Undo" action button for 5 seconds after successful toggle.
 * Covers: TC-5.1-01, TC-5.1-03 (Undo Toast)
 */
export function useToggleMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { path: string; enable: boolean }) => invoke<string>('toggle_mod', params),

    // Optimistic UI: flip is_enabled instantly in cache
    onMutate: async (variables) => {
      // Cancel outgoing refetches so they don't overwrite our optimistic update
      await queryClient.cancelQueries({ queryKey: folderKeys.all });

      // Snapshot previous folder lists for rollback
      const previousQueries = queryClient.getQueriesData<ModFolder[]>({
        queryKey: folderKeys.all,
      });

      // Optimistically flip the folder's is_enabled in every matching query
      queryClient.setQueriesData<ModFolder[]>({ queryKey: folderKeys.all }, (old) =>
        old?.map((f) => (f.path === variables.path ? { ...f, is_enabled: variables.enable } : f)),
      );

      return { previousQueries };
    },

    onSuccess: (newPath, variables) => {
      const action = variables.enable ? 'Enabled' : 'Disabled';
      toast.withAction('success', `${action} mod`, {
        label: 'Undo',
        onClick: () => {
          invoke<string>('toggle_mod', {
            path: newPath,
            enable: !variables.enable,
          }).then(() => {
            queryClient.invalidateQueries({ queryKey: folderKeys.all });
          });
        },
      });

      // After enabling: check for shader conflicts (non-blocking)
      if (variables.enable) {
        invoke<ConflictInfo[]>('check_shader_conflicts', { folderPath: newPath })
          .then((conflicts) => {
            if (conflicts.length > 0) {
              const names = conflicts.map((c) => c.mod_paths.join(', ')).join('; ');
              toast.info(`Shader collision detected: ${names}`);
            }
          })
          .catch(() => {
            /* non-critical — silently ignore */
          });
      }
    },

    onError: (error, _variables, context) => {
      // Rollback optimistic update on failure
      if (context?.previousQueries) {
        for (const [queryKey, data] of context.previousQueries) {
          queryClient.setQueryData(queryKey, data);
        }
      }
      toast.error(String(error));
    },

    // Always re-fetch after to get the real filesystem state
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

/** Hook to rename a mod folder. */
export function useRenameMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { folderPath: string; newName: string }) =>
      invoke<RenameResult>('rename_mod_folder', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

/** Hook to delete a mod folder to trash. */
export function useDeleteMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { path: string; gameId?: string }) => invoke<void>('delete_mod', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

/** Hook to restore a mod from trash. */
export function useRestoreMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (trashId: string) => invoke<string>('restore_mod', { trashId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['trash'] });
    },
  });
}

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

/** Hook to update a mod's category (object type). */
export function useUpdateModCategory() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { gameId: string; folderPath: string; category: string }) =>
      invoke<void>('set_mod_category', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
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
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.folder(variables.folderPath) });
    },
  });
}

/** Hook to delete a mod's thumbnail file. */
export function useDeleteModThumbnail() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (folderPath: string) => invoke<void>('delete_mod_thumbnail', { folderPath }),
    onSuccess: (_data, folderPath) => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
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
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.folder(variables.folderPath) });
    },
  });
}

/** Hook to bulk toggle mods. */
export function useBulkToggle() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { paths: string[]; enable: boolean }) =>
      invoke<BulkResult>('bulk_toggle_mods', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

/** Hook to bulk delete mods. */
export function useBulkDelete() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { paths: string[]; gameId?: string }) =>
      invoke<BulkResult>('bulk_delete_mods', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

/** Hook to bulk update info.json. */
/** Hook to bulk update info.json. */
export function useBulkUpdateInfo() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { paths: string[]; update: ModInfoUpdate }) =>
      invoke<BulkResult>('bulk_update_info', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export function useUpdateModInfo() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { folderPath: string; update: ModInfoUpdate }) =>
      invoke<ModInfo>('update_mod_info', params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export type ImportStrategy = 'Raw' | 'AutoOrganize';

/** Hook to import mods from external paths (Drag & Drop). */
export function useImportMods() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: {
      paths: string[];
      targetDir: string;
      strategy: ImportStrategy;
      dbJson?: string | null;
    }) => invoke<BulkResult>('import_mods_from_paths', params),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
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
    },
  });
}

// ── Epic 5: Advanced Mod Operations ────────────────────────────

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
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      const disabled = result.success.length - 1; // -1 for target itself
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
    staleTime: 5000,
  });
}
