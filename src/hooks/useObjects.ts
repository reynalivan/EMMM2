/**
 * Epic 3: TanStack Query hooks for object data fetching.
 * Uses objectService.ts for DB queries and invoke for schema loading.
 */

import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import type { GameType } from '../types/game';
import { useAppStore } from '../stores/useAppStore';
import { useActiveGame } from './useActiveGame';
import {
  getObjects,
  getCategoryCounts,
  updateObject,
  deleteObject,
  createObject,
} from '../lib/services/objectService';
import { corridorKeys } from '../features/collections/queryKeys';
import {
  type ObjectSummary,
  type ObjectFilter,
  type CategoryCount,
  type GameSchema,
  type UpdateObjectInput,
  type CreateObjectInput,
  ItemStatus,
} from '../types/object';

/** Query key factory for cache management */
const objectKeys = {
  all: ['objects'] as const,
  lists: () => [...objectKeys.all, 'list'] as const,
  list: (filter: ObjectFilter) => [...objectKeys.lists(), filter] as const,
  counts: (gameId: string) => [...objectKeys.all, 'counts', gameId] as const,
  schema: (gameType: GameType) => ['schema', gameType] as const,
};

interface UseObjectsOptions {
  metaFilters?: Record<string, string[]>;
  sortBy?: 'name' | 'date' | 'rarity';
  statusFilter?: 'all' | 'enabled' | 'disabled';
  /** When true, skip SQL search_query — search handled client-side by Web Worker */
  localSearch?: boolean;
}

/**
 * Fetch objects for the active game with current filters.
 * Covers: TC-3.1-01, TC-3.1-02
 */
export function useObjects(options: UseObjectsOptions = {}) {
  const { activeGame } = useActiveGame();
  const { safeMode, selectedObjectType, sidebarSearchQuery } = useAppStore();

  const filter: ObjectFilter = {
    game_id: activeGame?.id ?? '',
    safe_mode: safeMode,
    object_type: selectedObjectType ?? null,
    search_query: options.localSearch ? null : sidebarSearchQuery || null,
    meta_filters: options.metaFilters || null,
    sort_by: options.sortBy || null,
    status_filter:
      options.statusFilter === 'enabled'
        ? ItemStatus.Enabled
        : options.statusFilter === 'disabled'
          ? ItemStatus.Disabled
          : null,
  };

  // Use the full filter (including safe_mode) as the query key to ensure
  // TanStack Query ALWAYS fetches with the latest backend state.
  // placeholderData: (prev) => prev ensures the UI does not flash a loading spinner.
  const queryKeyFilter = filter;

  return useQuery<ObjectSummary[]>({
    queryKey: objectKeys.list(queryKeyFilter),
    queryFn: () => getObjects(filter),
    enabled: !!activeGame?.id,
    staleTime: 30_000,
    placeholderData: (prev) => prev,
    refetchOnWindowFocus: false, // Watcher handles external changes (req-28)
  });
}

/**
 * Fetch category counts (badges) for sidebar.
 * Covers: TC-3.1-02
 */
export function useCategoryCounts() {
  const { activeGame } = useActiveGame();
  const { safeMode } = useAppStore();
  const gameId = activeGame?.id ?? '';

  return useQuery<CategoryCount[]>({
    queryKey: [...objectKeys.counts(gameId), safeMode],
    queryFn: () => getCategoryCounts(gameId, safeMode),
    enabled: !!gameId,
    staleTime: 30_000,
    refetchOnWindowFocus: false, // Watcher handles external changes (req-28)
  });
}

/**
 * Fetch game schema from Rust backend.
 * Covers: NC-3.4-02 (Schema fallback)
 */
export function useGameSchema() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type;

  return useQuery<GameSchema>({
    queryKey: objectKeys.schema(gameType as GameType),
    queryFn: () => commands.getGameSchema({ gameType: gameType! }),
    enabled: gameType !== undefined,
    staleTime: Infinity, // Schemas don't change at runtime
  });
}

/**
 * Hook for game switching with cache invalidation.
 * Covers: TC-3.1-01, EC-3.01 (Rapid Switch)
 */
export function useGameSwitch() {
  const { setActiveGameId } = useAppStore();
  const queryClient = useQueryClient();

  const switchGame = async (gameId: string) => {
    await setActiveGameId(gameId);
    queryClient.invalidateQueries({ queryKey: objectKeys.all });
    queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
    queryClient.invalidateQueries({ queryKey: ['collections'] });
    queryClient.invalidateQueries({ queryKey: corridorKeys.all });
    queryClient.invalidateQueries({ queryKey: ['dashboard'] });
  };

  return { switchGame };
}

/**
 * Mutation: Update an object with cache invalidation.
 * Covers: TC-3.3-02
 */
export function useUpdateObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: UpdateObjectInput }) =>
      updateObject(id, updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: objectKeys.all });
    },
  });
}

/**
 * Mutation: Delete an object with cache invalidation.
 * Covers: NC-3.3-02
 */
export function useDeleteObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, force }: { id: string; force: boolean }) => deleteObject(id, force),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: objectKeys.all });
    },
  });
}

/**
 * Mutation: Create a new object with cache invalidation.
 * Covers: TC-3.3-01, NC-3.3-01
 */
export function useCreateObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateObjectInput) => createObject(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: objectKeys.all });
    },
  });
}

/**
 * Fetch MasterDB JSON for the active game.
 * Used for Smart Import in FolderGrid.
 */
export function useMasterDb() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type;

  return useQuery<string>({
    queryKey: ['master-db', gameType],
    queryFn: () => commands.getMasterDb({ gameType: gameType! }),
    enabled: !!gameType,
    staleTime: Infinity,
  });
}
