import type { QueryClient } from '@tanstack/react-query';
import { commands, type MatchedDbEntry } from '../../../lib/bindings';
import { updateFolderCache } from '../../../hooks/folderCache';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import type { ModFolder } from '../../../types/mod';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import { applyRuntimePathInvalidationMutationResult } from '../../workspace-runtime/actions/sharedRuntimeResultMapper';
import type { SyncCurrentData } from '../../workspace-runtime/state/workspaceState';

export interface SharedModSwitchActions {
  setNodeEnabled: (
    node: WorkspaceExplorerNode,
    enabled: boolean,
    surface: 'folder_grid' | 'preview' | 'object_list' | 'collections' | 'corridor',
    options: { syncExplorerPath: boolean },
  ) => Promise<string | null | undefined>;
}

export function hasIllegalCharacters(name: string): boolean {
  return /[\\/:*?"<>|]/.test(name);
}

export async function loadSharedModSyncMatch(params: {
  gameType: number;
  folder: ModFolder;
  currentData: SyncCurrentData;
}): Promise<MatchedDbEntry | null> {
  try {
    const match = await commands.matchObjectWithDb({
      gameType: params.gameType,
      objectName: params.folder.name,
    });
    return match ?? null;
  } catch {
    return null;
  }
}

export async function runSharedModActiveContextToggle(params: {
  activeGameId: string;
  folder: ModFolder;
  queryClient: QueryClient;
  removeFromCurrentView: boolean;
  switchSurface: 'folder_grid' | 'preview' | 'object_list' | 'collections' | 'corridor';
  switchActions: SharedModSwitchActions;
  hasPin: boolean;
  safeMode: boolean;
  translate: (key: string, vars?: Record<string, unknown>) => string;
}): Promise<{ kind: 'complete' } | { kind: 'requiresPinSafe'; folder: ModFolder }> {
  const newPath =
    (await params.switchActions.setNodeEnabled(
      params.folder as WorkspaceExplorerNode,
      false,
      params.switchSurface,
      {
        syncExplorerPath: false,
      },
    )) ?? params.folder.path;

  const targetSafeStatus = !params.folder.is_safe;
  if (params.safeMode && !targetSafeStatus && params.hasPin) {
    return {
      kind: 'requiresPinSafe',
      folder: { ...params.folder, path: newPath, is_enabled: false },
    };
  }

  await commands.toggleModSafe({
    gameId: params.activeGameId,
    folderPath: newPath,
    safe: targetSafeStatus,
  });

  if (params.removeFromCurrentView) {
    updateFolderCache(params.queryClient, [params.folder.path, newPath], undefined, true);
  }

  const store = useAppStore.getState();
  if (store.gridSelection?.has(params.folder.path) || store.gridSelection?.has(newPath)) {
    store.clearGridSelection();
  }

  toast.success(
    params.translate('objects:toasts.mark_safe_context', {
      context: targetSafeStatus
        ? params.translate('common:contexts.safe')
        : params.translate('common:contexts.unsafe'),
    }),
  );

  await applyRuntimePathInvalidationMutationResult(
    params.queryClient,
    [newPath],
    'workspaceStructure',
  );
  return { kind: 'complete' };
}
