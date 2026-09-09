import { commands } from '../../../shared/api/tauri/bindings';
import type { QueryClient, UseMutateAsyncFunction } from '@tanstack/react-query';
import type { GameConfig } from '@/entities/game';
import type { UpdateObjectInput } from '@/entities/game-object';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../optimistic/descriptorBuilders';

interface UpdateObjectVariables {
  id: string;
  updates: UpdateObjectInput;
}

interface UpdateObjectMutationLike {
  mutateAsync: UseMutateAsyncFunction<unknown, Error, UpdateObjectVariables, unknown>;
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
