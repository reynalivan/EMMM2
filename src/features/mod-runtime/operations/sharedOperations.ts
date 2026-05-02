import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import { updateFolderCache } from '../../../hooks/folderCache';
import type { GameConfig } from '../../../types/game';
import type { MasterDbEntry } from '../../object-list/scanReviewHelpers';
import type { MatchedDbEntry } from '../../../lib/bindings';
import {
  dispatchWorkspaceRuntimeEvent,
} from '../../workspace-runtime/state/workspaceStoreBridge';
import {
  applyRuntimeMutationResult,
} from '../../workspace-runtime/actions/sharedRuntimeResultMapper';
import { formatAppError } from '../../../lib/appError';

export function syncExplorerAfterRename(modPath: string, oldPath: string, newPath: string): void {
  const { explorerSubPath } = useAppStore.getState();
  if (!explorerSubPath) {
    return;
  }

  const clean = (path: string) => path.replace(/\\/g, '/');
  const cleanMod = clean(modPath);
  const cleanOld = clean(oldPath);
  const cleanNew = clean(newPath);
  const currentAbs = `${cleanMod}/${clean(explorerSubPath)}`;

  if (currentAbs !== cleanOld && !currentAbs.startsWith(`${cleanOld}/`)) {
    return;
  }

  const updated =
    currentAbs === cleanOld ? cleanNew : currentAbs.replace(`${cleanOld}/`, `${cleanNew}/`);
  let sub = updated.substring(cleanMod.length);
  if (sub.startsWith('/')) {
    sub = sub.substring(1);
  }
  if (!sub || sub === explorerSubPath) {
    return;
  }
  dispatchWorkspaceRuntimeEvent({
    type: 'PATHS_REWRITTEN',
    rewrites: [{ oldPath, newPath }],
  });
}

export function parseMasterDb(dbJson: string): MasterDbEntry[] {
  try {
    const parsed = JSON.parse(dbJson);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed.map((entry: Record<string, unknown>) => ({
      matched_entry_key:
        typeof entry.matched_entry_key === 'string' ? entry.matched_entry_key : '',
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
  removeFromCurrentView?: boolean;
}): Promise<void> {
  await commands.moveModToObject({
    gameId: params.gameId,
    folderPath: params.folderPath,
    targetObjectId: params.targetObjectId,
    status: params.status,
  });

  if (params.removeFromCurrentView) {
    updateFolderCache(params.queryClient, [params.folderPath], undefined, true);
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
