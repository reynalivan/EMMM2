import { formatAppError } from '../../../lib/appError';
import type { MoveStatus } from '../../../types/mod';
import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands, type MatchedDbEntry } from '../../../lib/bindings';
import { toast } from '../../../stores/useToastStore';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useSettings } from '../../../hooks/useSettings';
import { useSafeMode } from '../../../hooks/settingsQuery';
import { useBulkFavorite } from '../../../hooks/useBulkModMutations';
import { useToggleModSafe } from '../../../hooks/useFolderMutations';
import { useDeleteMod, useRenameMod } from '../../../hooks/useFolderCoreMutations';
import type { ModFolder } from '../../../types/object';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import {
  applyFolderDbSyncMatchAndRefresh,
  moveModsToObjectAndRefresh,
} from '../operations/sharedOperations';
import { useWorkspaceRuntimeSelector } from '../../workspace-runtime/state/workspaceStoreBridge';
import {
  useWorkspaceSwitchActions,
  type WorkspaceSwitchSurface,
} from '../../workspace-runtime/actions/useWorkspaceSwitchActions';
import { closeWorkspaceDialog } from '../../workspace-runtime/state/workspaceDialogs';
import {
  openModDialog,
  selectSharedModDialogState,
  updateModDialog,
} from './sharedModDialogs';
import {
  hasIllegalCharacters,
  loadSharedModSyncMatch,
  runSharedModActiveContextToggle,
} from './sharedModEffects';

// Dialog open/close take no closure state, so they live at module scope and keep
// a stable identity — these are spread into memoized card props.
const closeMoveDialog = () => closeWorkspaceDialog('modMove');
const closeSyncConfirm = () => closeWorkspaceDialog('modSync');
const handleDuplicateCancel = () => closeWorkspaceDialog('modDuplicateWarning');
const handleRenameCancel = () => closeWorkspaceDialog('modRename');
const handleToggleSafeCancel = () => closeWorkspaceDialog('modPinSafe');
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
  const safeMode = useSafeMode();

  const handleToggleEnabled = useCallback(
    async (folder: ModFolder) => {
      await switchActions.toggleNode(folder as WorkspaceExplorerNode, resolvedSwitchSurface);
    },
    [resolvedSwitchSurface, switchActions],
  );

  const handleDuplicateForceEnable = useCallback(
    (ignoreFuture: boolean = false) => {
      void (async () => {
        const { folder, duplicates } = state.duplicateWarning;

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
            toast.error(t('folder_grid:duplicate_warning.ignore_failed', { error: formatAppError(error) }));
          }
        }

        await switchActions.resolveDuplicateForceEnable(folder);
      })();
    },
    [state.duplicateWarning, activeGame, switchActions, t],
  );

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
      if (!activeGame) {
        return;
      }

      const currentData = {
        name: folder.name,
        object_type: folder.category ?? '',
        metadata: folder.metadata ?? null,
        thumbnail_path: folder.thumbnail_path,
      };

      openModDialog('modSync', { folder, match: null, isLoading: true, currentData });
      const match = await loadSharedModSyncMatch({
        gameType: activeGame.game_type,
        folder,
      });
      updateModDialog('modSync', { folder, match, isLoading: false, currentData });
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
        closeWorkspaceDialog('modSync');
      } catch (error) {
        toast.error(t('objects:edit_modal.error_message', { error: formatAppError(error) }));
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
        openModDialog('modActiveContext', { folder, isProcessing: false });
        return;
      }

      if (safeMode && folder.is_safe && hasPin) {
        openModDialog('modPinSafe', { folder });
        return;
      }

      toggleModSafe.mutate({
        gameId: activeGame.id,
        folderPath: folder.path,
        safe: !folder.is_safe,
      });
    },
    [activeGame, hasPin, safeMode, toggleModSafe],
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
    closeWorkspaceDialog('modPinSafe');
  }, [activeGame, state.pinSafeDialog.folder, toggleModSafe]);

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
        hasPin,
        safeMode,
        translate: t,
      });

      closeWorkspaceDialog('modActiveContext');
      if (outcome.kind === 'requiresPinSafe') {
        openModDialog('modPinSafe', { folder: outcome.folder });
      }
    } catch (error) {
      closeWorkspaceDialog('modActiveContext');
      toast.error(t('objects:create_modal.error_message', { error: formatAppError(error) }));
    }
  }, [
    activeGame,
    hasPin,
    queryClient,
    resolvedSwitchSurface,
    safeMode,
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
    openMoveDialog: (folder: ModFolder) => openModDialog('modMove', { folder }),
    closeMoveDialog,
    closeSyncConfirm,
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
    handleApplySyncMatch,
    handleToggleSafeRequest,
    handleToggleSafeSubmit,
    handleToggleSafeCancel,
    handleActiveContextCancel,
    handleActiveContextSubmit,
  };
}
