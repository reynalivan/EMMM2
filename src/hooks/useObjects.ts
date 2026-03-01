/**
 * Epic 3: TanStack Query hooks for object data fetching.
 * Uses objectService.ts for DB queries and invoke for schema loading.
 */

import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/useAppStore';
import { useActiveGame } from './useActiveGame';
import {
  getObjects,
  getCategoryCounts,
  updateObject,
  deleteObject,
  createObject,
} from '../lib/services/objectService';
import type {
  ObjectSummary,
  ObjectFilter,
  CategoryCount,
  GameSchema,
  UpdateObjectInput,
  CreateObjectInput,
} from '../types/object';

/** Query key factory for cache management */
const objectKeys = {
  all: ['objects'] as const,
  lists: () => [...objectKeys.all, 'list'] as const,
  list: (filter: ObjectFilter) => [...objectKeys.lists(), filter] as const,
  counts: (gameId: string, safe: boolean) => [...objectKeys.all, 'counts', gameId, safe] as const,
  schema: (gameType: string) => ['schema', gameType] as const,
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
    object_type: selectedObjectType ?? undefined,
    search_query: options.localSearch ? undefined : sidebarSearchQuery || undefined,
    meta_filters: options.metaFilters,
    sort_by: options.sortBy,
    status_filter: options.statusFilter,
  };

  return useQuery<ObjectSummary[]>({
    queryKey: objectKeys.list(filter),
    queryFn: () => getObjects(filter),
    enabled: !!activeGame?.id,
    staleTime: 30_000,
    placeholderData: (prev) => prev,
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
    queryKey: objectKeys.counts(gameId, safeMode),
    queryFn: () => getCategoryCounts(gameId, safeMode),
    enabled: !!gameId,
    staleTime: 30_000,
  });
}

/**
 * Fetch game schema from Rust backend.
 * Covers: NC-3.4-02 (Schema fallback)
 */
export function useGameSchema() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type ?? '';

  return useQuery<GameSchema>({
    queryKey: objectKeys.schema(gameType),
    queryFn: () => invoke<GameSchema>('get_game_schema', { gameType }),
    enabled: !!gameType,
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
    mutationFn: (id: string) => deleteObject(id),
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
  const gameType = activeGame?.game_type ?? '';

  return useQuery<string>({
    queryKey: ['master-db', gameType],
    queryFn: () => invoke<string>('get_master_db', { gameType }),
    enabled: !!gameType,
    staleTime: Infinity,
  });
}
