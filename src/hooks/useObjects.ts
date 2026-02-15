/**
 * Epic 3: TanStack Query hooks for object data fetching.
 * Uses objectService.ts for DB queries and invoke for schema loading.
 */

import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/useAppStore';
import {
  getObjects,
  getCategoryCounts,
  createObject,
  updateObject,
  deleteObject,
} from '../services/objectService';
import type {
  ObjectSummary,
  ObjectFilter,
  CategoryCount,
  GameSchema,
  CreateObjectInput,
  UpdateObjectInput,
} from '../types/object';

/** Query key factory for cache management */
const objectKeys = {
  all: ['objects'] as const,
  lists: () => [...objectKeys.all, 'list'] as const,
  list: (filter: ObjectFilter) => [...objectKeys.lists(), filter] as const,
  counts: (gameId: string, safe: boolean) => [...objectKeys.all, 'counts', gameId, safe] as const,
  schema: (gameType: string) => ['schema', gameType] as const,
};

/**
 * Fetch objects for the active game with current filters.
 * Covers: TC-3.1-01, TC-3.1-02
 */
export function useObjects() {
  const { activeGame, safeMode, selectedObjectType, sidebarSearchQuery } = useAppStore();

  const filter: ObjectFilter = {
    game_id: activeGame,
    safe_mode: safeMode,
    object_type: selectedObjectType ?? undefined,
    search_query: sidebarSearchQuery || undefined,
  };

  return useQuery<ObjectSummary[]>({
    queryKey: objectKeys.list(filter),
    queryFn: () => getObjects(filter),
    staleTime: 30_000,
    placeholderData: (prev) => prev,
  });
}

/**
 * Fetch category counts (badges) for sidebar.
 * Covers: TC-3.1-02
 */
export function useCategoryCounts() {
  const { activeGame, safeMode } = useAppStore();

  return useQuery<CategoryCount[]>({
    queryKey: objectKeys.counts(activeGame, safeMode),
    queryFn: () => getCategoryCounts(activeGame, safeMode),
    staleTime: 30_000,
  });
}

/**
 * Fetch game schema from Rust backend.
 * Covers: NC-3.4-02 (Schema fallback)
 */
export function useGameSchema() {
  const { activeGame } = useAppStore();

  return useQuery<GameSchema>({
    queryKey: objectKeys.schema(activeGame),
    queryFn: () => invoke<GameSchema>('get_game_schema', { gameType: activeGame }),
    staleTime: Infinity, // Schemas don't change at runtime
  });
}

/**
 * Hook for game switching with cache invalidation.
 * Covers: TC-3.1-01, EC-3.01 (Rapid Switch)
 */
export function useGameSwitch() {
  const { setActiveGame } = useAppStore();
  const queryClient = useQueryClient();

  const switchGame = async (gameType: string) => {
    await setActiveGame(gameType as 'GIMI' | 'SRMI' | 'ZZMI');
    queryClient.invalidateQueries({ queryKey: objectKeys.all });
  };

  return { switchGame };
}

/**
 * Mutation: Create an object with cache invalidation.
 * Covers: TC-3.3-01
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
