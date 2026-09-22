import { formatAppError } from '../../../shared/lib/appError';
import type { MoveStatus } from '@/entities/mod';
import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import { useActiveGame } from '@/entities/game';
import { useBulkFavorite } from '../hooks/useBulkModMutations';
import { useToggleModSafe } from '../hooks/useFolderMutations';
import { useDeleteMod, useRenameMod } from '../hooks/useFolderCoreMutations';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { moveModsToObjectAndRefresh } from '../operations/sharedOperations';
import { useWorkspaceRuntimeSelector } from '@/features/workspace-runtime/@x/mod-runtime';
import {
  useWorkspaceSwitchActions,
  type WorkspaceSwitchSurface,
} from '@/features/workspace-runtime/@x/mod-runtime';
import { closeWorkspaceDialog } from '@/features/workspace-runtime/@x/mod-runtime';
import { openModDialog, selectSharedModDialogState, updateModDialog } from './sharedModDialogs';
import { hasIllegalCharacters, runSharedModActiveContextToggle } from './sharedModEffects';
import { openObjectClassificationWizard } from '@/features/import-batches/@x/mod-runtime';

// Dialog open/close take no closure state, so they live at module scope and keep
// a stable identity — these are spread into memoized card props.
const closeMoveDialog = () => closeWorkspaceDialog('modMove');
const handleDuplicateCancel = () => closeWorkspaceDialog('modDuplicateWarning');
const handleRenameCancel = () => closeWorkspaceDialog('modRename');
const handleActiveContextCancel = () => closeWorkspaceDialog('modActiveContext');
const handleRenameRequest = (folder: ModFolder) => openModDialog('modRename', { folder });
const handleDeleteRequest = (folder: ModFolder) => openModDialog('modDelete', { folder });

interface SharedModActionsOptions {
  onRenameSuccess?: () => void;
  onDeleteSuccess?: () => void;
  onMoveSuccess?: () => void;
  switchSurface?: WorkspaceSwitchSurface;
}

export function useSharedModActions(options: SharedModActionsOptions = {}) {
  const { t } = useTranslation(['grid', 'objects', 'common', 'folder_grid']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const bulkFavorite = useBulkFavorite();
  const renameMod = useRenameMod();
  const deleteMod = useDeleteMod();
  const toggleModSafe = useToggleModSafe();
  const switchActions = useWorkspaceSwitchActions();
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);

  const state = useMemo(() => selectSharedModDialogState(dialogState), [dialogState]);
  const resolvedSwitchSurface = options.switchSurface ?? 'folder_grid';

  const handleToggleEnabled = useCallback(
    async (folder: ModFolder) => {
      await switchActions.toggleNode(folder as WorkspaceExplorerNode, resolvedSwitchSurface);
    },
    [resolvedSwitchSurface, switchActions],
  );

  const handleDuplicateForceEnable = useCallback(
    (ignoreFuture: boolean = false) => {
      void (async () => {
        const { folder, duplicates, enableDisabledAncestors, parentEnableConfirmation } =
          state.duplicateWarning;

        const nextPath = await switchActions.resolveDuplicateForceEnable(
          folder,
          enableDisabledAncestors,
          parentEnableConfirmation,
        );
        if (!nextPath) {
          return;
        }

        if (ignoreFuture && activeGame?.id && folder && duplicates.length > 0) {
          // Backend matches the ignore against the exact sorted set of
          // duplicate mod ids plus the target mod id — keep this in sync
          // with find duplicates in services/scanner/conflict.
          const modIds = duplicates.map((duplicate) => duplicate.mod_id);
          if (folder.id) {
            modIds.push(folder.id);
          }

          try {
            // No cache invalidation needed: the ignored-conflicts query is
            // gated on the management modal being open and refetches on open.
            await commands.ignoreObjectConflict(activeGame.id, duplicates[0].object_id, modIds);
          } catch (error) {
            toast.error(
              t('folder_grid:duplicate_warning.ignore_failed', { error: formatAppError(error) }),
            );
          }
        }
      })();
    },
    [state.duplicateWarning, activeGame, switchActions, t],
  );

  const handleDuplicateEnableOnly = useCallback(() => {
    void switchActions.resolveDuplicateEnableOnly(
      state.duplicateWarning.folder,
      state.duplicateWarning.enableDisabledAncestors,
      state.duplicateWarning.parentEnableConfirmation,
    );
  }, [
    state.duplicateWarning.enableDisabledAncestors,
    state.duplicateWarning.folder,
    state.duplicateWarning.parentEnableConfirmation,
    switchActions,
  ]);

  const handleEnableOnlyThis = useCallback(
    (folder: ModFolder) => {
      void switchActions.resolveDuplicateEnableOnly(folder);
    },
    [switchActions],
  );

  const handleToggleFavorite = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) {
        return;
      }

      bulkFavorite.mutate({
        gameId: activeGame.id,
        folderPaths: [folder.path],
        favorite: !folder.is_favorite,
      });
    },
    [activeGame, bulkFavorite],
  );

  const handleMoveToObject = useCallback(
    async (
      folder: ModFolder,
      targetObjectId: string,
      status: MoveStatus,
      targetSubpath?: string | null,
      targetModPaths?: string[],
    ) => {
      if (!activeGame?.id) {
        return;
      }

      await moveModsToObjectAndRefresh({
        queryClient,
        gameId: activeGame.id,
        folderPaths: targetModPaths && targetModPaths.length > 0 ? targetModPaths : [folder.path],
        targetObjectId,
        targetSubpath: targetSubpath ?? null,
        status,
      });
      options.onMoveSuccess?.();
    },
    [activeGame, options, queryClient],
  );

  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      const folder = state.renameDialog.folder;
      if (!folder || !activeGame?.id) {
        return;
      }

      if (hasIllegalCharacters(newName)) {
        toast.error(t('objects:edit_modal.validation.path_invalid'));
        return;
      }

      await renameMod.mutateAsync({
        folderPath: folder.path,
        newName,
        gameId: activeGame.id,
      });
      closeWorkspaceDialog('modRename');
      options.onRenameSuccess?.();
    },
    [activeGame, options, renameMod, state.renameDialog.folder, t],
  );

  const handleDeleteConfirm = useCallback(async () => {
    const folder = state.deleteConfirm.folder;
    if (!folder) {
      return;
    }

    if (!activeGame?.id) {
      return;
    }

    await deleteMod.mutateAsync({ path: folder.path, gameId: activeGame.id });
    closeWorkspaceDialog('modDelete');
    options.onDeleteSuccess?.();
  }, [activeGame, deleteMod, options, state.deleteConfirm.folder]);

  const setDeleteConfirm = useCallback((next: { open: boolean; folder: ModFolder | null }) => {
    if (next.open && next.folder) {
      openModDialog('modDelete', { folder: next.folder });
      return;
    }

    closeWorkspaceDialog('modDelete');
  }, []);

  const handleSyncWithDb = useCallback(
    async (folder: ModFolder) => {
      if (!activeGame || !folder.owner_object_id) {
        toast.warning(t('objects:classify_match.toast_none'));
        return;
      }
      openObjectClassificationWizard({
        gameId: activeGame.id,
        objectIds: [folder.owner_object_id],
      });
    },
    [activeGame, t],
  );

  const handleToggleSafeRequest = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) {
        return;
      }

      if (folder.is_enabled) {
        openModDialog('modActiveContext', { folder, isProcessing: false });
        return;
      }

      toggleModSafe.mutate({
        gameId: activeGame.id,
        folderPath: folder.path,
        safe: !folder.is_safe,
      });
    },
    [activeGame, toggleModSafe],
  );

  const handleActiveContextSubmit = useCallback(async () => {
    const folder = state.activeContextDialog.folder;
    if (!folder || !activeGame?.id) {
      return;
    }

    try {
      updateModDialog('modActiveContext', { folder, isProcessing: true });
      const outcome = await runSharedModActiveContextToggle({
        activeGameId: activeGame.id,
        folder,
        queryClient,
        switchSurface: resolvedSwitchSurface,
        switchActions: {
          setNodeEnabled: switchActions.setNodeEnabled,
        },
        translate: t,
      });

      closeWorkspaceDialog('modActiveContext');
      void outcome;
    } catch (error) {
      closeWorkspaceDialog('modActiveContext');
      toast.error(t('objects:create_modal.error_message', { error: formatAppError(error) }));
    }
  }, [
    activeGame,
    queryClient,
    resolvedSwitchSurface,
    state.activeContextDialog.folder,
    switchActions.setNodeEnabled,
    t,
  ]);

  return {
    moveDialog: state.moveDialog,
    renameDialog: state.renameDialog,
    deleteConfirm: state.deleteConfirm,
    activeContextDialog: state.activeContextDialog,
    duplicateWarning: state.duplicateWarning,
    isSwitchPending: switchActions.isPending,
    isFolderSwitchPending: switchActions.isNodePending,
    getPendingDesiredEnabled: switchActions.getPendingDesiredEnabled,
    setDeleteConfirm,
    openMoveDialog: (folder: ModFolder) => openModDialog('modMove', { folder }),
    closeMoveDialog,
    handleToggleEnabled,
    handleDuplicateForceEnable,
    handleDuplicateEnableOnly,
    handleDuplicateCancel,
    handleEnableOnlyThis,
    handleToggleFavorite,
    handleMoveToObject,
    handleRenameRequest,
    handleRenameSubmit,
    handleRenameCancel,
    handleDeleteRequest,
    handleDeleteConfirm,
    handleSyncWithDb,
    handleToggleSafeRequest,
    handleActiveContextCancel,
    handleActiveContextSubmit,
  };
}
