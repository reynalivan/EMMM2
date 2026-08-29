import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import type { ModFolder } from '../../../types/object';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import { applyRuntimePathInvalidationMutationResult } from '../../workspace-runtime/actions/sharedRuntimeResultMapper';
import { notifyCommittedMutationSyncWarning } from '../../../lib/committedMutationWarning';

export interface SharedModSwitchActions {
  setNodeEnabled: (
    node: WorkspaceExplorerNode,
    enabled: boolean,
    surface: 'folder_grid' | 'preview' | 'object_list' | 'collections',
  ) => Promise<string | null | undefined>;
}

export function hasIllegalCharacters(name: string): boolean {
  return /[\\/:*?"<>|]/.test(name);
}

export async function runSharedModActiveContextToggle(params: {
  activeGameId: string;
  folder: ModFolder;
  queryClient: QueryClient;
  switchSurface: 'folder_grid' | 'preview' | 'object_list' | 'collections';
  switchActions: SharedModSwitchActions;
  translate: (key: string, vars?: Record<string, unknown>) => string;
}): Promise<{ kind: 'complete' }> {
  const newPath =
    (await params.switchActions.setNodeEnabled(
      params.folder as WorkspaceExplorerNode,
      false,
      params.switchSurface,
    )) ?? params.folder.path;

  const targetSafeStatus = !params.folder.is_safe;
  const safetyResult = await commands.toggleModSafe(params.activeGameId, newPath, targetSafeStatus);
  notifyCommittedMutationSyncWarning(safetyResult);

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
