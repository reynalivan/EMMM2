import { useCallback, useMemo } from 'react';
import { join } from '@tauri-apps/api/path';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { toast } from '../../../stores/useToastStore';
import { useSharedModActions } from '../../mod-runtime/actions/useSharedModActions';
import {
  closeWorkspaceDialog,
  openWorkspaceEnableParentDialog,
} from '../../workspace-runtime/state/workspaceDialogs';
import { useWorkspaceSwitchActions } from '../../workspace-runtime/actions/useWorkspaceSwitchActions';
import { useWorkspaceRuntimeSelector } from '../../workspace-runtime/state/workspaceStoreBridge';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import type { ObjectSummary } from '../../../types/object';
import { applyRuntimeMutationResult } from '../../workspace-runtime/actions/sharedRuntimeResultMapper';

interface UseFolderGridActionsOptions {
  activeGame: ReturnType<typeof useActiveGame>['activeGame'];
  currentPath: string[];
  explorerSubPath: string | undefined;
  ancestorDisabledBy: string | null;
  ancestorDisabledPath: string | null;
  rawFolders: WorkspaceExplorerNode[];
  objects: ObjectSummary[];
  clearGridSelection: () => void;
}

export function useFolderGridActions({
  activeGame,
  currentPath,
  explorerSubPath,
  ancestorDisabledBy,
  ancestorDisabledPath,
  rawFolders,
  objects,
  clearGridSelection,
}: UseFolderGridActionsOptions) {
  const queryClient = useQueryClient();
  const actions = useSharedModActions({
    removeFromCurrentView: true,
    onRenameSuccess: clearGridSelection,
    onDeleteSuccess: clearGridSelection,
    onMoveSuccess: clearGridSelection,
    switchSurface: 'folder_grid',
  });
  const switchActions = useWorkspaceSwitchActions();
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);

  const enableParentDialog = useMemo(() => {
    if (dialogState.kind !== 'folderEnableParent') {
      return {
        open: false,
        ancestorName: '',
        willActivate: [] as WorkspaceExplorerNode[],
        stayDisabled: [] as WorkspaceExplorerNode[],
      };
    }

    return {
      open: true,
      ancestorName: dialogState.ancestorName,
      willActivate: dialogState.willActivate,
      stayDisabled: dialogState.stayDisabled,
    };
  }, [dialogState]);

  const currentAbsPath = useMemo(() => {
    if (!activeGame?.mod_path) {
      return null;
    }

    const parts = [activeGame.mod_path, ...currentPath.filter(Boolean)];
    return parts.join('\\');
  }, [activeGame, currentPath]);

  const handleRefresh = useCallback(() => {
    void applyRuntimeMutationResult(queryClient, 'workspaceStructure');
  }, [queryClient]);

  const handleRevealInExplorer = useCallback(
    async (objectId: string) => {
      if (!activeGame) {
        return;
      }

      const object = objects.find((candidate) => candidate.id === objectId);
      try {
        await commands.revealObjectInExplorer({
          gameId: activeGame.id,
          objectId,
          objectName: object?.folder_path ?? objectId,
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        toast.error(message);
        handleRefresh();
      }
    },
    [activeGame, handleRefresh, objects],
  );

  const handleOpenCurrentFolderInExplorer = useCallback(async () => {
    if (!currentAbsPath || !activeGame?.id) {
      return;
    }

    try {
      await commands.openInExplorer({ gameId: activeGame.id, path: currentAbsPath });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(message);
    }
  }, [activeGame, currentAbsPath]);

  const handleToggleSelf = useCallback(
    async (enable: boolean) => {
      if (!activeGame?.id || !activeGame.mod_path || !explorerSubPath) {
        return;
      }

      const targetPath = await join(activeGame.mod_path, explorerSubPath);
      await switchActions.setFolderPathEnabled(targetPath, enable, {
        syncExplorerPath: true,
      });
    },
    [activeGame, explorerSubPath, switchActions],
  );

  const openEnableParentDialog = useCallback(() => {
    if (!ancestorDisabledBy || !ancestorDisabledPath) {
      return;
    }

    const willActivate = rawFolders.filter((folder) => folder.is_enabled);
    const stayDisabled = rawFolders.filter((folder) => !folder.is_enabled);
    openWorkspaceEnableParentDialog({
      ancestorName: ancestorDisabledBy,
      ancestorPath: ancestorDisabledPath,
      willActivate,
      stayDisabled,
    });
  }, [ancestorDisabledBy, ancestorDisabledPath, rawFolders]);

  const closeEnableParentDialog = useCallback(() => {
    closeWorkspaceDialog('folderEnableParent');
  }, []);

  const handleEnableParent = useCallback(async () => {
    if (dialogState.kind !== 'folderEnableParent') {
      return;
    }

    await switchActions.setFolderPathEnabled(dialogState.ancestorPath, true, {
      syncExplorerPath: true,
    });
    closeWorkspaceDialog('folderEnableParent');
  }, [dialogState, switchActions]);

  const handleToggleEnabledGuarded = useCallback(
    (folder: WorkspaceExplorerNode) => {
      if (ancestorDisabledBy) {
        openEnableParentDialog();
        return;
      }

      void actions.handleToggleEnabled(folder);
    },
    [actions, ancestorDisabledBy, openEnableParentDialog],
  );

  return {
    actions,
    switchActions,
    enableParentDialog,
    handleRefresh,
    handleRevealInExplorer,
    currentAbsPath,
    handleOpenCurrentFolderInExplorer,
    handleToggleSelf,
    openEnableParentDialog,
    closeEnableParentDialog,
    handleEnableParent,
    handleToggleEnabledGuarded,
  };
}
