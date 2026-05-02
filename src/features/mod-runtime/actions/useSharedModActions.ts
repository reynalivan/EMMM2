import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import type { MatchedDbEntry } from '../../../lib/bindings';
import { toast } from '../../../stores/useToastStore';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useSettings } from '../../../hooks/useSettings';
import { useBulkFavorite, useToggleModSafe } from '../../../hooks/useFolderMutations';
import { useDeleteMod, useRenameMod } from '../../../hooks/useFolderCoreMutations';
import type { ModFolder } from '../../../types/mod';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import { useAppStore } from '../../../stores/useAppStore';
import {
  applyFolderDbSyncMatchAndRefresh,
  moveModToObjectAndRefresh,
} from '../operations/sharedOperations';
import { useWorkspaceRuntimeSelector } from '../../workspace-runtime/state/workspaceStoreBridge';
import {
  useWorkspaceSwitchActions,
  type WorkspaceSwitchSurface,
} from '../../workspace-runtime/actions/useWorkspaceSwitchActions';
import {
  closeModActiveContextDialog,
  closeModDeleteDialog,
  closeModDuplicateWarningDialog,
  closeModMoveDialog,
  closeModPinSafeDialog,
  closeModRenameDialog,
  closeModSyncDialog,
  openModActiveContextDialog,
  openModDeleteDialog,
  openModMoveDialog,
  openModPinSafeDialog,
  openModRenameDialog,
  openModSyncDialog,
  selectSharedModDialogState,
  updateModActiveContextDialog,
  updateModSyncDialog,
} from './sharedModDialogs';
import {
  hasIllegalCharacters,
  loadSharedModSyncMatch,
  runSharedModActiveContextToggle,
} from './sharedModEffects';

interface SharedModActionsOptions {
  removeFromCurrentView?: boolean;
  onRenameSuccess?: () => void;
  onDeleteSuccess?: () => void;
  onMoveSuccess?: () => void;
  switchSurface?: WorkspaceSwitchSurface;
}

export function useSharedModActions(options: SharedModActionsOptions = {}) {
  const { t } = useTranslation(['grid', 'objects', 'common']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const { settings } = useSettings();
  const bulkFavorite = useBulkFavorite();
  const renameMod = useRenameMod();
  const deleteMod = useDeleteMod();
  const toggleModSafe = useToggleModSafe();
  const switchActions = useWorkspaceSwitchActions();
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);

  const state = useMemo(() => selectSharedModDialogState(dialogState), [dialogState]);
  const resolvedSwitchSurface = options.switchSurface ?? 'folder_grid';
  const hasPin = useMemo(() => !!settings?.safe_mode?.pin_hash, [settings?.safe_mode?.pin_hash]);

  const handleToggleEnabled = useCallback(
    async (folder: ModFolder) => {
      await switchActions.toggleNode(folder as WorkspaceExplorerNode, resolvedSwitchSurface, {
        syncExplorerPath: false,
      });
    },
    [resolvedSwitchSurface, switchActions],
  );

  const handleDuplicateForceEnable = useCallback(() => {
    void switchActions.resolveDuplicateForceEnable(state.duplicateWarning.folder);
  }, [state.duplicateWarning.folder, switchActions]);

  const handleDuplicateEnableOnly = useCallback(() => {
    void switchActions.resolveDuplicateEnableOnly(state.duplicateWarning.folder);
  }, [state.duplicateWarning.folder, switchActions]);

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
      status: 'disabled' | 'only-enable' | 'keep',
    ) => {
      if (!activeGame?.id) {
        return;
      }

      await moveModToObjectAndRefresh({
        queryClient,
        gameId: activeGame.id,
        folderPath: folder.path,
        targetObjectId,
        status,
        removeFromCurrentView: options.removeFromCurrentView,
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
      closeModRenameDialog();
      options.onRenameSuccess?.();
    },
    [activeGame, options, renameMod, state.renameDialog.folder, t],
  );

  const handleDeleteConfirm = useCallback(async () => {
    const folder = state.deleteConfirm.folder;
    if (!folder) {
      return;
    }

    await deleteMod.mutateAsync({ path: folder.path, gameId: activeGame?.id });
    closeModDeleteDialog();
    options.onDeleteSuccess?.();
  }, [activeGame, deleteMod, options, state.deleteConfirm.folder]);

  const setDeleteConfirm = useCallback((next: { open: boolean; folder: ModFolder | null }) => {
    if (next.open && next.folder) {
      openModDeleteDialog(next.folder);
      return;
    }

    closeModDeleteDialog();
  }, []);

  const handleSyncWithDb = useCallback(
    async (folder: ModFolder) => {
      if (!activeGame) {
        return;
      }

      const currentData = {
        name: folder.name,
        object_type: folder.category ?? '',
        metadata: folder.metadata ?? null,
        thumbnail_path: folder.thumbnail_path,
      };

      openModSyncDialog(folder, currentData);
      const match = await loadSharedModSyncMatch({
        gameType: activeGame.game_type,
        folder,
        currentData,
      });
      updateModSyncDialog(folder, currentData, match, false);
    },
    [activeGame],
  );

  const handleApplySyncMatch = useCallback(
    async (match: MatchedDbEntry) => {
      const folder = state.syncConfirm.folder;
      if (!folder || !activeGame) {
        return;
      }

      try {
        await applyFolderDbSyncMatchAndRefresh({
          queryClient,
          activeGame,
          folderPath: folder.path,
          match,
        });
        toast.success(t('objects:edit_modal.success_message', { name: folder.name }));
        closeModSyncDialog();
      } catch (error) {
        toast.error(t('objects:edit_modal.error_message', { error: String(error) }));
      }
    },
    [activeGame, queryClient, state.syncConfirm.folder, t],
  );

  const handleToggleSafeRequest = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) {
        return;
      }

      if (folder.is_enabled) {
        openModActiveContextDialog(folder);
        return;
      }

      const safeMode = useAppStore.getState().safeMode;
      if (safeMode && folder.is_safe && hasPin) {
        openModPinSafeDialog(folder);
        return;
      }

      toggleModSafe.mutate({
        gameId: activeGame.id,
        folderPath: folder.path,
        safe: !folder.is_safe,
      });
    },
    [activeGame, hasPin, toggleModSafe],
  );

  const handleToggleSafeSubmit = useCallback(() => {
    const folder = state.pinSafeDialog.folder;
    if (!folder || !activeGame?.id) {
      return;
    }

    toggleModSafe.mutate({
      gameId: activeGame.id,
      folderPath: folder.path,
      safe: false,
    });
    closeModPinSafeDialog();
  }, [activeGame, state.pinSafeDialog.folder, toggleModSafe]);

  const handleActiveContextSubmit = useCallback(async () => {
    const folder = state.activeContextDialog.folder;
    if (!folder || !activeGame?.id) {
      return;
    }

    try {
      updateModActiveContextDialog(folder, true);
      const outcome = await runSharedModActiveContextToggle({
        activeGameId: activeGame.id,
        folder,
        queryClient,
        removeFromCurrentView: options.removeFromCurrentView ?? false,
        switchSurface: resolvedSwitchSurface,
        switchActions: {
          setNodeEnabled: switchActions.setNodeEnabled,
        },
        hasPin,
        safeMode: useAppStore.getState().safeMode,
        translate: t,
      });

      closeModActiveContextDialog();
      if (outcome.kind === 'requiresPinSafe') {
        openModPinSafeDialog(outcome.folder);
      }
    } catch (error) {
      closeModActiveContextDialog();
      toast.error(t('objects:create_modal.error_message', { error: String(error) }));
    }
  }, [
    activeGame,
    hasPin,
    options,
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
    pinSafeDialog: state.pinSafeDialog,
    activeContextDialog: state.activeContextDialog,
    duplicateWarning: state.duplicateWarning,
    syncConfirm: state.syncConfirm,
    isSwitchPending: switchActions.isPending,
    isFolderSwitchPending: switchActions.isNodePending,
    hasPin,
    setDeleteConfirm,
    openMoveDialog: openModMoveDialog,
    closeMoveDialog: closeModMoveDialog,
    closeSyncConfirm: closeModSyncDialog,
    handleToggleEnabled,
    handleDuplicateForceEnable,
    handleDuplicateEnableOnly,
    handleDuplicateCancel: closeModDuplicateWarningDialog,
    handleEnableOnlyThis,
    handleToggleFavorite,
    handleMoveToObject,
    handleRenameRequest: openModRenameDialog,
    handleRenameSubmit,
    handleRenameCancel: closeModRenameDialog,
    handleDeleteRequest: openModDeleteDialog,
    handleDeleteConfirm,
    handleSyncWithDb,
    handleApplySyncMatch,
    handleToggleSafeRequest,
    handleToggleSafeSubmit,
    handleToggleSafeCancel: closeModPinSafeDialog,
    handleActiveContextCancel: closeModActiveContextDialog,
    handleActiveContextSubmit,
  };
}
