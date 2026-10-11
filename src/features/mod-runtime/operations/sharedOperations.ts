import type { MoveStatus } from '@/entities/mod';
import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import { applyRuntimeMutationResult } from '@/features/workspace-runtime/@x/mod-runtime';
import { applyRuntimeEffects } from '@/features/workspace-runtime/@x/mod-runtime';
import {
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '@/features/workspace-runtime/@x/mod-runtime';
import { mergeRuntimeEffectDescriptors } from '@/features/workspace-runtime/@x/mod-runtime';
import { thumbnailKeys } from '@/entities/mod';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';
import {
  openBulkFileInUseRetryDialog,
  openBulkFileLockErrorDialog,
} from '@/features/workspace-runtime/@x/mod-runtime';

interface MoveAndRefreshParams {
  queryClient: QueryClient;
  gameId: string;
  folderPaths: string[];
  targetObjectId: string;
  targetSubpath: string | null;
  status: MoveStatus;
  recoveryIds?: string[];
}

async function performMoveAndRefresh(params: MoveAndRefreshParams) {
  const result = await commands.moveModsToObject(
    {
      game_id: params.gameId,
      folder_paths: params.folderPaths,
      target_object_id: params.targetObjectId,
      target_subpath: params.targetSubpath,
      status: params.status,
    },
    params.recoveryIds ?? null,
  );

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
  return result;
}

export async function moveModsToObjectAndRefresh(params: MoveAndRefreshParams): Promise<void> {
  const variables = { ...params, paths: params.folderPaths };
  const retry = (retryVariables: typeof variables & { recoveryIds: string[] }) =>
    performMoveAndRefresh({ ...retryVariables, folderPaths: retryVariables.paths });
  let result;
  try {
    result = await performMoveAndRefresh(params);
  } catch (error) {
    openBulkFileLockErrorDialog(error, variables, retry);
    throw error;
  }
  if (result.failures.length > 0) {
    openBulkFileInUseRetryDialog(result, variables, retry);
    throw result.failures[0].error;
  }
}
