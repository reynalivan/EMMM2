import type { QueryClient } from '@tanstack/react-query';
import { publishRuntimeDescriptor, type QueryRefetchType } from '../features/runtime-sync/queryRefresh';
import { buildRuntimeRefreshDescriptor } from '../features/workspace-runtime/optimistic/descriptorBuilders';
import { workspaceKeys } from '../features/workspace-runtime/useWorkspaceViewModel';
import type {
  CategoryCount,
  CreateObjectInput,
  GameSchema,
  ObjectFilter,
  ObjectSummary,
  UpdateObjectInput,
} from '../types/object';
import type { GameType } from '../types/game';
import type { WorkspaceObjectNode, WorkspaceViewModel } from '../types/workspace';

export const objectKeys = {
  all: ['objects'] as const,
  lists: () => [...objectKeys.all, 'list'] as const,
  list: (filter: ObjectFilter) => [...objectKeys.lists(), filter] as const,
  countsAll: () => [...objectKeys.all, 'counts'] as const,
  counts: (gameId: string) => [...objectKeys.all, 'counts', gameId] as const,
  schema: (gameType: GameType) => ['schema', gameType] as const,
};

export interface ObjectListSnapshot {
  objectQueries: Array<readonly [readonly unknown[], ObjectSummary[] | undefined]>;
  workspaceQueries: Array<readonly [readonly unknown[], WorkspaceViewModel | undefined]>;
}

export interface ObjectListRefreshOptions {
  includeFolders?: boolean;
  includeCorridor?: boolean;
  includeCollections?: boolean;
  includeDashboard?: boolean;
  includeActiveKeybindings?: boolean;
  refetchType?: QueryRefetchType;
  folderRefetchType?: QueryRefetchType;
}

type ObjectSummaryUpdater = (object: ObjectSummary) => ObjectSummary;
type WorkspaceObjectNodeUpdater = (object: WorkspaceObjectNode) => WorkspaceObjectNode;

export function buildObjectListRefreshDescriptor(options: ObjectListRefreshOptions = {}) {
  const events = ['workspaceChanged'] as Array<
    | 'workspaceChanged'
    | 'corridorChanged'
    | 'collectionsChanged'
    | 'dashboardChanged'
    | 'activeKeybindingsChanged'
    | 'folderStructureChanged'
  >;

  if (options.includeCorridor) {
    events.push('corridorChanged');
  }
  if (options.includeCollections) {
    events.push('collectionsChanged');
  }
  if (options.includeDashboard) {
    events.push('dashboardChanged');
  }
  if (options.includeActiveKeybindings) {
    events.push('activeKeybindingsChanged');
  }
  if (options.includeFolders) {
    events.push('folderStructureChanged');
  }

  return buildRuntimeRefreshDescriptor(events);
}

function serializeMetadata(metadata: UpdateObjectInput['metadata'], fallback: string): string {
  if (metadata === undefined) {
    return fallback;
  }
  if (metadata === null) {
    return '{}';
  }

  return JSON.stringify(metadata);
}

function serializeTags(tags: UpdateObjectInput['tags'], fallback: string): string {
  if (tags === undefined) {
    return fallback;
  }
  if (tags === null) {
    return '[]';
  }

  return JSON.stringify(tags);
}

export function patchObjectSummary(object: ObjectSummary, updates: UpdateObjectInput): ObjectSummary {
  return {
    ...object,
    name: updates.name ?? object.name,
    object_type: updates.object_type ?? object.object_type,
    sub_category: updates.sub_category ?? object.sub_category,
    status: updates.status ?? object.status,
    metadata: serializeMetadata(updates.metadata, object.metadata),
    tags: serializeTags(updates.tags, object.tags),
    hash_db: updates.hash_db === undefined ? object.hash_db : updates.hash_db,
    custom_skins:
      updates.custom_skins === undefined ? object.custom_skins : updates.custom_skins,
    thumbnail_path:
      updates.thumbnail_path === undefined ? object.thumbnail_path : updates.thumbnail_path,
    is_auto_sync: updates.is_auto_sync ?? object.is_auto_sync,
  };
}

export function snapshotObjectListQueries(queryClient: QueryClient): ObjectListSnapshot {
  return {
    objectQueries: queryClient.getQueriesData<ObjectSummary[]>({ queryKey: objectKeys.lists() }),
    workspaceQueries: queryClient.getQueriesData<WorkspaceViewModel>({
      queryKey: workspaceKeys.all,
    }),
  };
}

export function restoreObjectListQueries(
  queryClient: QueryClient,
  snapshot: ObjectListSnapshot,
): void {
  for (const [queryKey, data] of snapshot.objectQueries) {
    queryClient.setQueryData(queryKey, data);
  }
  for (const [queryKey, data] of snapshot.workspaceQueries) {
    queryClient.setQueryData(queryKey, data);
  }
}

export function patchObjectListQueries(
  queryClient: QueryClient,
  objectId: string,
  updater: ObjectSummaryUpdater,
): void {
  queryClient.setQueriesData<ObjectSummary[]>({ queryKey: objectKeys.lists() }, (current) => {
    if (!current) {
      return current;
    }

    return current.map((object) => (object.id === objectId ? updater(object) : object));
  });
  queryClient.setQueriesData<WorkspaceViewModel>({ queryKey: workspaceKeys.all }, (current) => {
    if (!current) {
      return current;
    }

    return {
      ...current,
      objects: current.objects.map((object) => {
        if (object.id !== objectId) {
          return object;
        }

        const patchedObject = updater(object);
        return {
          ...object,
          ...patchedObject,
          display_name: patchedObject.name,
        };
      }),
    };
  });
}

function clampEnabledCount(enabledCount: number, modCount: number): number {
  if (enabledCount < 0) {
    return 0;
  }
  if (enabledCount > modCount) {
    return modCount;
  }

  return enabledCount;
}

export function patchObjectEnabledCount(
  queryClient: QueryClient,
  objectId: string,
  delta: number,
): void {
  patchObjectListQueries(queryClient, objectId, (object) => ({
    ...object,
    enabled_count: clampEnabledCount(object.enabled_count + delta, object.mod_count),
  }));
}

interface ObjectBatchMutationOptions {
  queryClient: QueryClient;
  applyOptimisticUpdate: (object: ObjectSummary) => ObjectSummary;
  applyWorkspaceOptimisticUpdate?: WorkspaceObjectNodeUpdater;
  mutation: () => Promise<void>;
  refreshOptions?: ObjectListRefreshOptions;
}

export async function runObjectBatchMutation({
  queryClient,
  applyOptimisticUpdate,
  applyWorkspaceOptimisticUpdate,
  mutation,
  refreshOptions,
}: ObjectBatchMutationOptions): Promise<void> {
  const snapshot = snapshotObjectListQueries(queryClient);
  queryClient.setQueriesData<ObjectSummary[]>({ queryKey: objectKeys.lists() }, (current) => {
    if (!current) {
      return current;
    }

    return current.map(applyOptimisticUpdate);
  });
  queryClient.setQueriesData<WorkspaceViewModel>({ queryKey: workspaceKeys.all }, (current) => {
    if (!current) {
      return current;
    }

    return {
      ...current,
      objects: current.objects.map((object) => {
        if (applyWorkspaceOptimisticUpdate) {
          return applyWorkspaceOptimisticUpdate(object);
        }

        const patchedObject = applyOptimisticUpdate(object);
        return {
          ...object,
          ...patchedObject,
          display_name: patchedObject.name,
        };
      }),
    };
  });

  try {
    await mutation();
  } catch (error) {
    restoreObjectListQueries(queryClient, snapshot);
    throw error;
  }

  await publishRuntimeDescriptor(
    queryClient,
    buildObjectListRefreshDescriptor(refreshOptions),
    'active',
  );
}

export type {
  CategoryCount,
  CreateObjectInput,
  GameSchema,
  ObjectFilter,
  ObjectSummary,
  UpdateObjectInput,
};
