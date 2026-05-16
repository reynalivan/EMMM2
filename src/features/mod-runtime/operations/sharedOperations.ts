import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { toast } from '../../../stores/useToastStore';
import { updateFolderCache } from '../../../hooks/folderCache';
import type { GameConfig } from '../../../types/game';
import type { MasterDbEntry } from '../../object-list/scanReviewHelpers';
import type { MatchedDbEntry } from '../../../lib/bindings';
import { applyRuntimeMutationResult } from '../../workspace-runtime/actions/sharedRuntimeResultMapper';
import { formatAppError } from '../../../lib/appError';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import {
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../../workspace-runtime/optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../../workspace-runtime/optimistic/descriptor';
import { thumbnailKeys } from '../../../hooks/useThumbnail';

export function parseMasterDb(dbJson: string): MasterDbEntry[] {
  try {
    const parsed = JSON.parse(dbJson);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed.map((entry: Record<string, unknown>) => ({
      matched_entry_key: typeof entry.matched_entry_key === 'string' ? entry.matched_entry_key : '',
      name: String(entry.name ?? ''),
      object_type: String(entry.object_type ?? 'Other'),
      tags: Array.isArray(entry.tags) ? (entry.tags as string[]) : [],
      metadata: (entry.metadata as Record<string, unknown>) ?? null,
      thumbnail_path: entry.thumbnail_path ? String(entry.thumbnail_path) : null,
    }));
  } catch {
    return [];
  }
}

export async function moveModToObjectAndRefresh(params: {
  queryClient: QueryClient;
  gameId: string;
  folderPath: string;
  targetObjectId: string;
  status: 'disabled' | 'only-enable' | 'keep';
  targetSubpath?: string | null;
  removeFromCurrentView?: boolean;
}): Promise<void> {
  await moveModsToObjectAndRefresh({
    queryClient: params.queryClient,
    gameId: params.gameId,
    folderPaths: [params.folderPath],
    targetObjectId: params.targetObjectId,
    targetSubpath: params.targetSubpath ?? null,
    status: params.status,
    removeFromCurrentView: params.removeFromCurrentView,
  });
}

export async function moveModsToObjectAndRefresh(params: {
  queryClient: QueryClient;
  gameId: string;
  folderPaths: string[];
  targetObjectId: string;
  targetSubpath: string | null;
  status: 'disabled' | 'only-enable' | 'keep';
  removeFromCurrentView?: boolean;
}): Promise<void> {
  const result = await commands.moveModsToObject({
    input: {
      game_id: params.gameId,
      folder_paths: params.folderPaths,
      target_object_id: params.targetObjectId,
      target_subpath: params.targetSubpath,
      status: params.status,
    },
  });

  if (params.removeFromCurrentView) {
    updateFolderCache(params.queryClient, params.folderPaths, undefined, true);
  }

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

  if (result.failures.length > 0) {
    throw result.failures[0].error;
  }

  await applyRuntimeMutationResult(params.queryClient, 'workspaceStructure');
}

export async function applyFolderDbSyncMatchAndRefresh(params: {
  queryClient: QueryClient;
  activeGame: GameConfig;
  folderPath: string;
  match: MatchedDbEntry;
}): Promise<void> {
  await commands.applyObjectMatch({
    input: {
      game_id: params.activeGame.id,
      folder_path: params.folderPath,
      matched_entry_key: params.match.matched_entry_key ?? null,
      matched_alias_name: params.match.matched_alias_name ?? params.match.name,
      matched_reason: params.match.match_detail,
      matched_source: 'manual_match',
    },
  });

  if (params.match.object_type) {
    await commands.setModCategory({
      gameId: params.activeGame.id,
      folderPath: params.folderPath,
      category: params.match.object_type,
    });
    await applyRuntimeMutationResult(params.queryClient, 'workspaceStructure');
  }

  if (params.match.metadata) {
    const metaStrings: Record<string, string> = {};
    Object.entries(params.match.metadata).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        metaStrings[key] = String(value);
      }
    });
    await commands.updateModInfo({
      folderPath: params.folderPath,
      update: { metadata: metaStrings },
    });
  }

  if (params.match.thumbnail_path) {
    await commands.updateModThumbnail({
      folderPath: params.folderPath,
      sourcePath: params.match.thumbnail_path,
    });
  }

  await applyRuntimeMutationResult(params.queryClient, 'folderMetadataThumbnail');
}

export async function executeImportAndInvalidate(
  paths: string[],
  targetDir: string,
  queryClient: QueryClient,
  options: {
    isNewObject?: boolean;
    objectName?: string;
  },
): Promise<void> {
  const result = await commands.importModsFromPaths({
    paths,
    targetDir,
    strategy: 'Raw',
    dbJson: undefined,
  });

  await applyRuntimeMutationResult(queryClient, 'workspaceStructure');

  const movedCount = result.success.length;
  const failCount = result.failures.length;
  if (movedCount > 0) {
    const fails = failCount > 0 ? `, ${failCount} failed` : '';
    const label = options.isNewObject
      ? `Created ${options.objectName ?? 'Object'} with ${movedCount} item(s)${fails}`
      : `Moved ${movedCount} item(s)${options.objectName ? ` to ${options.objectName}` : ''}${fails}`;
    toast.success(label);
    return;
  }
  if (failCount > 0) {
    const action = options.isNewObject
      ? 'Created Object but failed to move items'
      : 'Failed to move items';
    toast.error(`${action}: ${formatAppError(result.failures[0].error)}`);
  }
}
