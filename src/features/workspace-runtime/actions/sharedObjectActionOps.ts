import { commands, sparse, type MatchedDbEntry } from '../../../lib/bindings';
import type { QueryClient, UseMutateAsyncFunction } from '@tanstack/react-query';
import type { GameConfig } from '../../../types/game';
import type { JsonValue, UpdateObjectInput } from '../../../types/object';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import type { ObjectSyncCurrentData } from './sharedObjectActionsState';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../optimistic/descriptorBuilders';

interface UpdateObjectVariables {
  id: string;
  updates: UpdateObjectInput;
}

interface UpdateObjectMutationLike {
  mutateAsync: UseMutateAsyncFunction<unknown, Error, UpdateObjectVariables, unknown>;
}

export function buildObjectSyncCurrentData(
  object: WorkspaceObjectNode | undefined,
  objectName: string,
): ObjectSyncCurrentData {
  if (!object) {
    return {
      name: objectName,
      object_type: '',
      metadata: null,
      thumbnail_path: null,
    };
  }

  return {
    name: object.name,
    object_type: object.object_type,
    metadata: null,
    thumbnail_path: object.thumbnail_path,
  };
}

export async function applyObjectCategoryAndRefresh(params: {
  activeGame: GameConfig;
  objectId: string;
  category: string;
  itemType: 'object' | 'folder';
  queryClient: QueryClient;
  updateObject: UpdateObjectMutationLike;
}): Promise<void> {
  if (params.itemType === 'folder') {
    await commands.setModCategory(params.activeGame.id, params.objectId, params.category);
  } else {
    await params.updateObject.mutateAsync({
      id: params.objectId,
      updates: { object_type: params.category },
    });

    await commands.setObjectModsCategory(params.activeGame.id, params.objectId, params.category);
  }

  await publishRuntimeDescriptor(
    params.queryClient,
    buildRuntimeMutationDescriptor('objectStructure', ['folderMetadataChanged']),
    'active',
  );
}

export async function revealObjectInExplorer(params: {
  activeGame: GameConfig;
  objectId: string;
  objectFolderPath: string | undefined;
}): Promise<void> {
  await commands.revealObjectInExplorer(
    params.activeGame.id,
    params.objectId,
    params.objectFolderPath ?? params.objectId,
  );
}

export async function loadObjectSyncMatch(params: {
  activeGame: GameConfig;
  objectName: string;
}): Promise<MatchedDbEntry | null> {
  const match = await commands.matchObjectWithDb(params.activeGame.game_type, params.objectName);

  return match ?? null;
}

export async function applyObjectSyncMatch(params: {
  activeGame: GameConfig;
  objectId: string;
  match: MatchedDbEntry;
  updateObject: UpdateObjectMutationLike;
}): Promise<void> {
  await commands.applyObjectMatchCmd(
    sparse({
      game_id: params.activeGame.id,
      object_id: params.objectId,
      matched_entry_key: params.match.matched_entry_key ?? null,
      matched_alias_name: params.match.matched_alias_name ?? params.match.name,
      matched_reason: params.match.match_detail,
      matched_source: 'manual_match',
    }),
  );

  await params.updateObject.mutateAsync({
    id: params.objectId,
    updates: {
      object_type: params.match.object_type,
      metadata: (params.match.metadata ?? undefined) as JsonValue | undefined,
      thumbnail_path: params.match.thumbnail_path ?? undefined,
    },
  });
}
