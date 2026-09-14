import { formatAppError } from '../../../shared/lib/appError';
import { useCallback, useMemo, useState } from 'react';
import { join } from '@tauri-apps/api/path';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import { useActiveGame } from '@/entities/game';
import { toast } from '@/shared/ui/toast';
import { useSharedModActions } from '@/features/mod-runtime';
import { useWorkspaceSwitchActions } from '@/features/workspace-runtime';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import type { ObjectSummary } from '@/entities/game-object';
import { applyRuntimeMutationResult } from '@/features/workspace-runtime';

interface UseFolderGridActionsOptions {
  activeGame: ReturnType<typeof useActiveGame>['activeGame'];
  explorerSubPath: string | undefined;
  ancestorDisabledPath: string | null;
  objects: ObjectSummary[];
  clearGridSelection: () => void;
  sourceAvailable: boolean;
}

interface CreateFolderTarget {
  gameId: string;
  parentPath: string;
}

export function useFolderGridActions({
  activeGame,
  explorerSubPath,
  ancestorDisabledPath,
  objects,
  clearGridSelection,
  sourceAvailable,
}: UseFolderGridActionsOptions) {
  const queryClient = useQueryClient();
  const actions = useSharedModActions({
    onRenameSuccess: clearGridSelection,
    onDeleteSuccess: clearGridSelection,
    onMoveSuccess: clearGridSelection,
    switchSurface: 'folder_grid',
  });
  const switchActions = useWorkspaceSwitchActions();
  const activeGameId = activeGame?.id;
  const [createFolderTarget, setCreateFolderTarget] = useState<CreateFolderTarget | null>(null);
  const [isCreatingFolder, setIsCreatingFolder] = useState(false);

  // `currentPath` is a display breadcrumb. `explorerSubPath` is the canonical
  // workspace path and remains correct when an object is nested under a group.
  const currentFolderPath = useMemo(() => {
    if (!sourceAvailable || !activeGameId) {
      return null;
    }

    return explorerSubPath ?? '';
  }, [activeGameId, explorerSubPath, sourceAvailable]);

  const refreshWorkspaceQueries = useCallback(() => {
    void applyRuntimeMutationResult(queryClient, 'workspaceStructure');
  }, [queryClient]);

  const handleRevealInExplorer = useCallback(
    async (objectId: string) => {
      if (!activeGame) {
        return;
      }

      const object = objects.find((candidate) => candidate.id === objectId);
      try {
        await commands.revealObjectInExplorer(
          activeGame.id,
          objectId,
          object?.folder_path ?? objectId,
        );
      } catch (error) {
        const message = formatAppError(error);
        toast.error(message);
        refreshWorkspaceQueries();
      }
    },
    [activeGame, objects, refreshWorkspaceQueries],
  );

  const handleOpenCurrentFolderInExplorer = useCallback(async () => {
    if (currentFolderPath === null || !activeGameId) {
      return;
    }

    try {
      await commands.openInExplorer(activeGameId, currentFolderPath);
    } catch (error) {
      const message = formatAppError(error);
      toast.error(message);
    }
  }, [activeGameId, currentFolderPath]);

  const openCreateFolderDialog = useCallback(() => {
    if (currentFolderPath !== null && activeGameId) {
      setCreateFolderTarget({ gameId: activeGameId, parentPath: currentFolderPath });
    }
  }, [activeGameId, currentFolderPath]);

  const closeCreateFolderDialog = useCallback(() => {
    if (!isCreatingFolder) {
      setCreateFolderTarget(null);
    }
  }, [isCreatingFolder]);

  const handleCreateFolder = useCallback(
    async (folderName: string) => {
      if (!createFolderTarget) {
        return;
      }

      setIsCreatingFolder(true);
      try {
        await commands.createModFolder(
          createFolderTarget.parentPath,
          folderName,
          createFolderTarget.gameId,
        );
        clearGridSelection();
        await applyRuntimeMutationResult(queryClient, 'workspaceStructure');
        toast.success(`Created folder "${folderName}"`);
      } catch (error) {
        toast.error(`Could not create folder: ${formatAppError(error)}`);
        throw error;
      } finally {
        setIsCreatingFolder(false);
      }
    },
    [clearGridSelection, createFolderTarget, queryClient],
  );

  const handleToggleSelf = useCallback(
    async (enable: boolean) => {
      if (!sourceAvailable || !activeGame?.id || !activeGame.mod_path || !explorerSubPath) {
        return;
      }

      const targetPath = await join(activeGame.mod_path, explorerSubPath);
      await switchActions.setFolderPathEnabled(targetPath, enable);
    },
    [activeGame, explorerSubPath, sourceAvailable, switchActions],
  );

  const openEnableParentDialog = useCallback(() => {
    if (!ancestorDisabledPath) {
      return;
    }
    void switchActions.setFolderPathEnabled(ancestorDisabledPath, true);
  }, [ancestorDisabledPath, switchActions]);

  const handleToggleEnabledGuarded = useCallback(
    (folder: WorkspaceExplorerNode) => {
      void actions.handleToggleEnabled(folder);
    },
    [actions],
  );

  return {
    actions,
    switchActions,
    handleRevealInExplorer,
    currentFolderPath,
    handleOpenCurrentFolderInExplorer,
    isCreateFolderOpen: createFolderTarget !== null,
    isCreatingFolder,
    openCreateFolderDialog,
    closeCreateFolderDialog,
    handleCreateFolder,
    handleToggleSelf,
    openEnableParentDialog,
    handleToggleEnabledGuarded,
  };
}
