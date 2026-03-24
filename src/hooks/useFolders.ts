/**
 * Hook for fetching raw mod folder listings from the filesystem.
 * Used as the sidebar/explorer data source.
 * Enhanced for Epic 4 with sub_path navigation, mutations, and sorting.
 */

import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import type { ModInfo } from '../types/object';
import { toast } from '../stores/useToastStore';
import { useAppStore } from '../stores/useAppStore';
import { useActiveGame } from './useActiveGame';
import { thumbnailKeys } from './useThumbnail';
import { corridorKeys } from '../features/collections/queryKeys';
import type {
  ModFolder,
  FolderGridResponse,
  SortField,
  SortOrder,
  DuplicateInfo,
} from '../types/mod';

// Re-export for consumers that import from here
export type { ModFolder, FolderGridResponse, ModInfo };

/** Query key factory for folder cache management */
export const folderKeys = {
  all: ['mod-folders'] as const,
  list: (modsPath: string, subPath?: string, safeMode?: boolean) =>
    [...folderKeys.all, modsPath, subPath ?? '', safeMode ?? null] as const,
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
export function useModFolders(subPath?: string, objectId?: string | null) {
  const { activeGame } = useActiveGame();
  const { safeMode } = useAppStore();
  const modsPath = activeGame?.mod_path;
  const gameId = activeGame?.id;

  return useQuery<FolderGridResponse>({
    queryKey: [...folderKeys.list(modsPath ?? '', subPath, safeMode), objectId],
    queryFn: () =>
      commands.listModFolders({
        gameId: gameId!,
        modsPath: modsPath!,
        subPath: subPath || undefined,
        objectId: objectId || null,
      }),
    enabled: !!modsPath && !!gameId,
    staleTime: 30_000,
    refetchOnWindowFocus: false,
    retry: 2,
    placeholderData: keepPreviousData,
  });
}

/** Sort folders client-side by field and direction.
 * AC-12.3.2: Sort is applied *within* each visual group:
 *   Group 1 — ContainerFolders (navigable directories)
 *   Group 2 — ModPackRoot / VariantContainer / FlatModRoot (mod entries)
 */
export function sortFolders(folders: ModFolder[], field: SortField, order: SortOrder): ModFolder[] {
  const sortGroup = (group: ModFolder[]): ModFolder[] => {
    const sorted = [...group].sort((a, b) => {
      // Favorites always on top within their group
      if (a.is_favorite !== b.is_favorite) {
        return a.is_favorite ? -1 : 1;
      }

      const cmp = (() => {
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
      })();

      // Secondary sort by name to ensure stable, deterministic order (AC-12.3.5)
      return cmp !== 0 ? cmp : a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    });
    return order === 'desc' ? sorted.reverse() : sorted;
  };

  const containers = folders.filter((f) => f.node_type === 'ContainerFolder');
  const packs = folders.filter((f) => f.node_type !== 'ContainerFolder');

  return [...sortGroup(containers), ...sortGroup(packs)];
}

/** Helper to trigger full GC + Sync fallback when a file operation fails due to missing files */
async function fallbackSync(queryClient: ReturnType<typeof useQueryClient>, gameId: string | null) {
  if (!gameId) return;
  toast.info('Syncing changes from disk...', 3000);
  try {
    await commands.gcLostObjects({ gameId });
    await commands.syncObjects({ gameId });
    toast.success('Sync complete', 2000);
  } catch (err) {
    console.error('Fallback sync failed:', err);
    toast.error('Sync failed', 3000);
  } finally {
    queryClient.invalidateQueries({ queryKey: folderKeys.all });
    queryClient.invalidateQueries({ queryKey: ['objects'] });
    queryClient.invalidateQueries({ queryKey: ['category-counts'] });
    queryClient.invalidateQueries({ queryKey: corridorKeys.all });
  }
}

/**
 * Toggle a mod's enabled/disabled state with undo toast.
 * Shows an "Undo" action button for 5 seconds after successful toggle.
 * Covers: TC-5.1-01, TC-5.1-03 (Undo Toast)
 */
export function useToggleMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: {
      path: string;
      enable: boolean;
      gameId: string;
      suppressToast?: boolean;
    }) =>
      commands.toggleMod({
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

    onSuccess: async (newPath, variables) => {
      queryClient.removeQueries({ queryKey: thumbnailKeys.folder(variables.path) }); // Invalidate old thumbnail URI
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });

      const action = variables.enable ? 'Enabled' : 'Disabled';
      let modName = variables.path.split(/[/\\]/).pop() || 'mod';
      if (modName.startsWith('DISABLED ')) {
        modName = modName.substring(9);
      }

      // Try to find the actual display name from the cache
      const prevQueries = queryClient.getQueriesData<FolderGridResponse>({
        queryKey: folderKeys.all,
      });
      for (const [, data] of prevQueries) {
        if (!data) continue;
        const match = data.children.find((f) => f.path === variables.path);
        if (match) {
          modName = match.name;
          break;
        }
      }

      // Opt-Z1: Fix path instability. The folder was physically renamed on disk.
      // We must update its path in the cache so subsequent actions (like Delete) don't use the old stale path.
      updateFolderCache(queryClient, [variables.path], (f) => ({
        ...f,
        path: newPath,
        is_enabled: variables.enable,
      }));

      // Opt-Z2: Update grid selection to reflect new path instantly
      const store = useAppStore.getState();
      store.replaceGridSelection(variables.path, newPath);
      store.correctExplorerPath(variables.path, newPath);

      if (!variables.suppressToast) {
        toast.withAction('success', `${action} ${modName}`, {
          label: 'Undo',
          onClick: () => {
            useAppStore.getState().setWatcherCooldown(Date.now() + 500);
            commands
              .toggleMod({
                path: newPath,
                enable: !variables.enable,
                gameId: variables.gameId,
              })
              .then(() => {
                queryClient.removeQueries({ queryKey: thumbnailKeys.folder(newPath) });
                queryClient.invalidateQueries({ queryKey: folderKeys.all });
                queryClient.invalidateQueries({ queryKey: ['objects'] });
                queryClient.invalidateQueries({ queryKey: corridorKeys.all });
              });
          },
        });
      }

      if (variables.enable) {
        commands
          .checkShaderConflicts({ folderPath: newPath })
          .then(() => queryClient.invalidateQueries({ queryKey: ['conflicts'] }))
          .catch(() => {
            /* non-critical — silently ignore */
          });
      }
    },

    onError: (error, variables, context) => {
      // Rollback optimistic update on failure
      if (context?.previousQueries) {
        for (const [queryKey, data] of context.previousQueries) {
          queryClient.setQueryData(queryKey, data);
        }
      }
      const errStr = String(error);

      // Detect "File Not Found" errors to trigger Fallback Sync
      if (
        errStr.toLowerCase().includes('not found') ||
        errStr.toLowerCase().includes('os error 2')
      ) {
        fallbackSync(queryClient, variables.gameId);
        return;
      }

      // Detect structured RenameConflict error → open Resolve dialog
      if (errStr.includes('"type":"RenameConflict"')) {
        try {
          const conflict = JSON.parse(errStr);
          useAppStore.getState().openConflictDialog(conflict);
          return;
        } catch {
          /* parse failed, fall through to generic toast */
        }
      }

      // Detect structured DuplicateConflict error → open Resolve dialog
      if (errStr.includes('"type":"DuplicateConflict"')) {
        try {
          const body = JSON.parse(errStr);
          const duplicates = body.content as DuplicateInfo[];
          const folder = queryClient
            .getQueriesData<FolderGridResponse>({ queryKey: folderKeys.all })
            .flatMap(([, data]) => data?.children || [])
            .find((f) => f.path === variables.path);

          if (folder) {
            useAppStore.getState().openDuplicateConflictDialog(folder, duplicates);
            return;
          }
        } catch {
          /* parse failed */
        }
      }
      // Detect structured FileInUse error → open FileInUse dialog
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

      const fallbackErrStr = typeof error === 'object' ? JSON.stringify(error) : String(error);
      toast.error(fallbackErrStr);

    },

    // Mark data as stale — don't force immediate refetch.
    // Optimistic update already shows the correct state in cache.
    onSettled: () => {
      // Suppress leaked watcher events for 500ms after mutation settles
      useAppStore.getState().setWatcherCooldown(Date.now() + 500);
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'active' });
      queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['conflicts'], refetchType: 'none' });
    },
  });
}

/** Hook to rename a mod folder. */
export function useRenameMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: { folderPath: string; newName: string; gameId: string }) =>
      commands.renameModFolder({
        folderPath: params.folderPath,
        newName: params.newName,
        gameId: params.gameId,
      }),

    onSuccess: async (result, variables) => {
      queryClient.removeQueries({ queryKey: thumbnailKeys.folder(variables.folderPath) });

      // Opt-Z2: Fix path instability. The folder was physically renamed.
      // We must update both its name and its path so it stays interactive in the current view.
      updateFolderCache(queryClient, [variables.folderPath], (f) => ({
        ...f,
        name: result.new_name,
        path: result.new_path,
      }));

      // Update grid selection to reflect new path instantly
      const store = useAppStore.getState();
      store.replaceGridSelection(variables.folderPath, result.new_path);
      store.correctExplorerPath(variables.folderPath, result.new_path);

      // Path changed — mark stale, refetch deferred until next user interaction
      // Suppress leaked watcher events for 500ms after rename settles
      useAppStore.getState().setWatcherCooldown(Date.now() + 500);
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['conflicts'], refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all, refetchType: 'none' });
    },
    onError: (error, variables) => {
      const errStr = String(error);

      // Detect structured FileInUse error → open FileInUse dialog
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

      if (
        errStr.toLowerCase().includes('not found') ||
        errStr.toLowerCase().includes('os error 2')
      ) {

        fallbackSync(queryClient, variables.gameId);
      } else {
        toast.error(`Rename failed: ${errStr}`);
      }
    },
  });
}

/** Hook to delete a mod folder to trash. */
export function useDeleteMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: { path: string; gameId?: string }) => commands.deleteMod(params),
    onSuccess: async (_, variables) => {
      queryClient.removeQueries({ queryKey: thumbnailKeys.folder(variables.path) });
      // Targeted: remove deleted folder from cache instead of full re-listing
      updateFolderCache(queryClient, [variables.path], undefined, true);
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['conflicts'], refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
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

      if (
        errStr.toLowerCase().includes('not found') ||
        errStr.toLowerCase().includes('os error 2')
      ) {

        fallbackSync(queryClient, variables.gameId ?? null);
      } else {
        toast.error(`Delete failed: ${errStr}`);
      }
    },
  });
}

/** Hook to restore a mod from trash. */
export function useRestoreMod() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: { trashId: string; gameId?: string }) =>
      commands.restoreMod({ trashId: params.trashId, gameId: params.gameId }),

    onSuccess: async () => {
      // New folder appears on disk — active refetch needed (no optimistic data)
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['trash'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
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

      toast.error(`Restore failed: ${errStr}`);
    },
  });

  return mutation;
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
