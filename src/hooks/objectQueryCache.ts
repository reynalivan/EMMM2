import type { QueryClient } from '@tanstack/react-query';
import {
  publishRuntimeDescriptor,
  type QueryRefetchType,
} from '../features/runtime-sync/queryRefresh';
import { buildRefreshDescriptor } from '../features/workspace-runtime/optimistic/descriptorBuilders';
import type { RuntimeRefreshEvent } from '../lib/runtimeEffects';
import type {
  CategoryCount,
  CreateObjectInput,
  GameSchema,
  ObjectFilter,
  UpdateObjectInput,
} from '../types/object';
import type { GameType } from '../types/game';

export const objectKeys = {
  all: ['objects'] as const,
  lists: () => [...objectKeys.all, 'list'] as const,
  list: (filter: ObjectFilter) => [...objectKeys.lists(), filter] as const,
  counts: (gameId: string) => [...objectKeys.all, 'counts', gameId] as const,
  schema: (gameType: GameType) => ['schema', gameType] as const,
};

export interface ObjectListRefreshOptions {
  includeFolders?: boolean;
  includeCorridor?: boolean;
  includeCollections?: boolean;
  includeDashboard?: boolean;
  refetchType?: QueryRefetchType;
}

export function buildObjectListRefreshDescriptor(options: ObjectListRefreshOptions = {}) {
  const events: RuntimeRefreshEvent[] = ['workspaceChanged'];

  if (options.includeCorridor) {
    events.push('corridorChanged');
  }
  if (options.includeCollections) {
    events.push('collectionsChanged');
  }
  if (options.includeDashboard) {
    events.push('dashboardChanged');
  }
  if (options.includeFolders) {
    events.push('folderStructureChanged');
  }

  return buildRefreshDescriptor(events);
}

interface ObjectBatchMutationOptions {
  queryClient: QueryClient;
  mutation: () => Promise<void>;
  refreshOptions?: ObjectListRefreshOptions;
}

/**
 * Runs a bulk object mutation and schedules the refetches that make its result
 * visible. Invalidation-only: no optimistic cache patching, so a thrown
 * mutation needs no rollback — the caller reports it and the caches were never
 * touched.
 */
export async function runObjectBatchMutation({
  queryClient,
  mutation,
  refreshOptions,
}: ObjectBatchMutationOptions): Promise<void> {
  await mutation();
  await publishRuntimeDescriptor(
    queryClient,
    buildObjectListRefreshDescriptor(refreshOptions),
    'active',
  );
}

export type { CategoryCount, CreateObjectInput, GameSchema, UpdateObjectInput };
