import type { MoveStatus } from '../../../types/mod';
import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { applyRuntimeMutationResult } from '../../workspace-runtime/actions/sharedRuntimeResultMapper';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../../workspace-runtime/optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../../workspace-runtime/optimistic/descriptor';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import { notifyCommittedMutationSyncWarning } from '../../../lib/committedMutationWarning';

export async function moveModsToObjectAndRefresh(params: {
  queryClient: QueryClient;
  gameId: string;
  folderPaths: string[];
  targetObjectId: string;
  targetSubpath: string | null;
  status: MoveStatus;
}): Promise<void> {
  const result = await commands.moveModsToObject({
    game_id: params.gameId,
    folder_paths: params.folderPaths,
    target_object_id: params.targetObjectId,
    target_subpath: params.targetSubpath,
    status: params.status,
  });

  applyRuntimeEffects(
    params.queryClient,
    mergeRuntimeEffectDescriptors(
      buildQueryRemovalDescriptor(
        result.path_rewrites.map((rewrite) => thumbnailKeys.folder(rewrite.old_path)),
        [],
      ),
      buildWorkspacePathRewritesDescriptor(result.path_rewrites, []),
    ),
  );
  await applyRuntimeMutationResult(params.queryClient, 'workspaceStructure');
  notifyCommittedMutationSyncWarning(result);

  if (result.failures.length > 0) {
    throw result.failures[0].error;
  }
}
