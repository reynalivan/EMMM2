/**
 * Hook for fetching raw mod folder listings from the filesystem.
 * Used as the sidebar/explorer data source.
 * Enhanced for Epic 4 with sub_path navigation, mutations, and sorting.
 */

import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../stores/useToastStore';
import { useAppStore } from '../stores/useAppStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import type {
  ModFolder,
  FolderGridResponse,
  RenameResult,
  SortField,
  SortOrder,
  ModInfo,
  ConflictInfo,
} from '../types/mod';

// Re-export for consumers that import from here
export type { ModFolder, FolderGridResponse, ModInfo };

/** Query key factory for folder cache management */
export const folderKeys = {
  all: ['mod-folders'] as const,
  list: (modsPath: string, subPath?: string, objectId?: string) =>
    [...folderKeys.all, modsPath, subPath ?? '', objectId ?? ''] as const,
};

/**
 * Helper to update folders in the query cache instead of full refetch.
 * Supports updating fields or removing the folder entirely.
 */
export function updateFolderCache(
  queryClient: import('@tanstack/react-query').QueryClient,
  pathsToUpdate: string[],
  updater?: (folder: ModFolder) => ModFolder,
  remove: boolean = false,
) {
  if (pathsToUpdate.length === 0) return;

  const queries = queryClient.getQueriesData<FolderGridResponse>({ queryKey: folderKeys.all });
  queries.forEach(([queryKey, data]) => {
    if (!data) return;

    let updatedChildren;
    if (remove) {
      updatedChildren = data.children.filter((f) => !pathsToUpdate.includes(f.path));
    } else if (updater) {
      updatedChildren = data.children.map((f) => (pathsToUpdate.includes(f.path) ? updater(f) : f));
    } else {
      updatedChildren = data.children;
    }

    queryClient.setQueryData(queryKey, {
      ...data,
      children: updatedChildren,
    });
  });
}

/**
 * Fetch mod folders from the active game's mods_path directory.
 * Supports sub_path for deep navigation and objectId for DB-level filtering by object.
 */
export function useModFolders(subPath?: string, objectId?: string) {
  const { activeGame } = useActiveGame();
  const modsPath = activeGame?.mod_path;

  return useQuery<FolderGridResponse>({
    queryKey: folderKeys.list(modsPath ?? '', subPath, objectId),
    queryFn: () =>
      invoke<FolderGridResponse>('list_mod_folders', {
        gameId: activeGame?.id ?? null,
        modsPath: modsPath!,
        subPath: subPath || null,
        objectId: objectId || null,
      }),
    enabled: !!modsPath,
    staleTime: 30_000,
    refetchOnWindowFocus: false,
    retry: 2,
    placeholderData: keepPreviousData,
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
    mutationFn: (params: { path: string; enable: boolean; gameId: string }) =>
      invoke<string>('toggle_mod', {
        path: params.path,
        enable: params.enable,
        gameId: params.gameId,
      }),

    // Optimistic UI: flip is_enabled instantly in cache
    onMutate: async (variables) => {
      // Cancel outgoing refetches so they don't overwrite our optimistic update
      await queryClient.cancelQueries({ queryKey: folderKeys.all });

      // Snapshot previous folder lists for rollback
      const previousQueries = queryClient.getQueriesData<FolderGridResponse>({
        queryKey: folderKeys.all,
      });

      // Optimistically flip the folder's is_enabled in every matching query
      // Also flip self_is_enabled if the toggled path is the folder itself (FlatModRoot)
      queryClient.setQueriesData<FolderGridResponse>({ queryKey: folderKeys.all }, (old) => {
        if (!old) return old;
        const childMatch = old.children.some((f) => f.path === variables.path);
        return {
          ...old,
          children: old.children.map((f) =>
            f.path === variables.path ? { ...f, is_enabled: variables.enable } : f,
          ),
          // If no child matched, this folder itself is being toggled (FlatModRoot case)
          self_is_enabled: childMatch ? old.self_is_enabled : variables.enable,
        };
      });

      return { previousQueries };
    },

    onSuccess: (newPath, variables) => {
      const action = variables.enable ? 'Enabled' : 'Disabled';
      queryClient.removeQueries({ queryKey: thumbnailKeys.folder(variables.path) }); // Invalidate old thumbnail URI

      // Opt-Z1: Fix path instability. The folder was physically renamed on disk.
      // We must update its path in the cache so subsequent actions (like Delete) don't use the old stale path.
      updateFolderCache(queryClient, [variables.path], (f) => ({
        ...f,
        path: newPath,
        is_enabled: variables.enable,
      }));

      toast.withAction('success', `${action} mod`, {
        label: 'Undo',
        onClick: () => {
          useAppStore.getState().setWatcherCooldown(Date.now() + 500);
          invoke<string>('toggle_mod', {
            path: newPath,
            enable: !variables.enable,
            gameId: variables.gameId,
          }).then(() => {
            queryClient.removeQueries({ queryKey: thumbnailKeys.folder(newPath) });
            queryClient.invalidateQueries({ queryKey: folderKeys.all });
            queryClient.invalidateQueries({ queryKey: ['objects'] });
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
              queryClient.invalidateQueries({ queryKey: ['conflicts'] });
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
      // Detect structured RenameConflict error → open Resolve dialog
      const errStr = String(error);
      if (errStr.includes('"type":"RenameConflict"')) {
        try {
          const conflict = JSON.parse(errStr);
          useAppStore.getState().openConflictDialog(conflict);
          return;
        } catch {
          /* parse failed, fall through to generic toast */
        }
      }
      toast.error(String(error));
    },

    // Mark data as stale — don't force immediate refetch.
    // Optimistic update already shows the correct state in cache.
    onSettled: () => {
      // Suppress leaked watcher events for 500ms after mutation settles
      useAppStore.getState().setWatcherCooldown(Date.now() + 500);
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['conflicts'], refetchType: 'none' });
    },
  });
}

/** Hook to rename a mod folder. */
export function useRenameMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { folderPath: string; newName: string; gameId: string }) =>
      invoke<RenameResult>('rename_mod_folder', {
        folder_path: params.folderPath,
        new_name: params.newName,
        game_id: params.gameId,
      }),
    onSuccess: (result, variables) => {
      queryClient.removeQueries({ queryKey: thumbnailKeys.folder(variables.folderPath) });

      // Opt-Z2: Fix path instability. The folder was physically renamed.
      // We must update both its name and its path so it stays interactive in the current view.
      updateFolderCache(queryClient, [variables.folderPath], (f) => ({
        ...f,
        name: result.new_name,
        path: result.new_path,
      }));

      // Path changed — mark stale, refetch deferred until next user interaction
      // Suppress leaked watcher events for 500ms after rename settles
      useAppStore.getState().setWatcherCooldown(Date.now() + 500);
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'none' });
    },
  });
}

/** Hook to delete a mod folder to trash. */
export function useDeleteMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { path: string; gameId?: string }) => invoke<void>('delete_mod', params),
    onSuccess: (_, variables) => {
      queryClient.removeQueries({ queryKey: thumbnailKeys.folder(variables.path) });
      // Targeted: remove deleted folder from cache instead of full re-listing
      updateFolderCache(queryClient, [variables.path], undefined, true);
      queryClient.invalidateQueries({ queryKey: ['objects'] });
    },
  });
}

/** Hook to restore a mod from trash. */
export function useRestoreMod() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { trashId: string; gameId?: string }) =>
      invoke<string>('restore_mod', { trashId: params.trashId, gameId: params.gameId }),
    onSuccess: () => {
      // New folder appears on disk — active refetch needed (no optimistic data)
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['trash'] });
    },
  });
}

// ── Barrel re-exports ──────────────────────────────────────────
// Consumers keep importing from './useFolders' — zero migration needed.
export {
  trashKeys,
  useListTrash,
  useEmptyTrash,
  useUpdateModCategory,
  useUpdateModThumbnail,
  useToggleModSafe,
  useDeleteModThumbnail,
  usePasteThumbnail,
  useUpdateModInfo,
  useBulkToggle,
  useBulkDelete,
  useBulkUpdateInfo,
  useBulkFavorite,
  useBulkPin,
  useImportMods,
  useAutoOrganizeMods,
  useEnableOnlyThis,
  useCheckDuplicate,
  useActiveConflicts,
} from './useFolderMutations';
export type { ImportStrategy } from './useFolderMutations';
