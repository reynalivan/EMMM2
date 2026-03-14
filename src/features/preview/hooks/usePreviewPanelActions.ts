import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../../hooks/useActiveGame';
import {
  useToggleMod,
  useRenameMod,
  useDeleteMod,
  useEnableOnlyThis,
  useCheckDuplicate,
  useToggleModSafe,
  useBulkFavorite,
  updateFolderCache,
  ModFolder,
} from '../../../hooks/useFolders';
import type { DuplicateInfo } from '../../../types/mod';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';

export function usePreviewPanelActions() {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  const toggleMod = useToggleMod();
  const renameMod = useRenameMod();
  const deleteMod = useDeleteMod();
  const enableOnlyThis = useEnableOnlyThis();
  const checkDuplicate = useCheckDuplicate();
  const toggleModSafe = useToggleModSafe();
  const bulkFavorite = useBulkFavorite();

  const [moveDialog, setMoveDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
    open: false,
    folder: null,
  });

  const [pinSafeDialog, setPinSafeDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
    open: false,
    folder: null,
  });

  const [renameDialog, setRenameDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
    open: false,
    folder: null,
  });

  const [deleteConfirm, setDeleteConfirm] = useState<{
    open: boolean;
    folder: ModFolder | null;
  }>({ open: false, folder: null });

  const [duplicateWarning, setDuplicateWarning] = useState<{
    open: boolean;
    folder: ModFolder | null;
    duplicates: DuplicateInfo[];
  }>({ open: false, folder: null, duplicates: [] });

  const handleToggleEnabled = useCallback(
    async (folder: ModFolder) => {
      if (!activeGame?.id) return;

      if (folder.is_enabled) {
        toggleMod.mutate({ path: folder.path, enable: false, gameId: activeGame.id });
        return;
      }

      try {
        const duplicates = await checkDuplicate.mutateAsync({
          folderPath: folder.path,
          gameId: activeGame.id,
        });

        if (duplicates.length > 0) {
          setDuplicateWarning({ open: true, folder, duplicates });
          return;
        }
      } catch {
        // Proceed if duplicate check fails
      }

      toggleMod.mutate({ path: folder.path, enable: true, gameId: activeGame.id });
    },
    [toggleMod, activeGame, checkDuplicate],
  );

  const handleDuplicateForceEnable = useCallback(() => {
    if (!duplicateWarning.folder || !activeGame?.id) return;
    toggleMod.mutate({ path: duplicateWarning.folder.path, enable: true, gameId: activeGame.id });
    setDuplicateWarning({ open: false, folder: null, duplicates: [] });
  }, [duplicateWarning.folder, toggleMod, activeGame]);

  const handleDuplicateEnableOnly = useCallback(() => {
    if (!duplicateWarning.folder || !activeGame?.id) return;
    enableOnlyThis.mutate({
      targetPath: duplicateWarning.folder.path,
      gameId: activeGame.id,
    });
    setDuplicateWarning({ open: false, folder: null, duplicates: [] });
  }, [duplicateWarning.folder, activeGame, enableOnlyThis]);

  const handleDuplicateCancel = useCallback(() => {
    setDuplicateWarning({ open: false, folder: null, duplicates: [] });
  }, []);

  const handleEnableOnlyThis = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;
      enableOnlyThis.mutate({ targetPath: folder.path, gameId: activeGame.id });
    },
    [activeGame, enableOnlyThis],
  );

  const handleToggleFavorite = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;
      bulkFavorite.mutate({
        gameId: activeGame.id,
        folderPaths: [folder.path],
        favorite: !folder.is_favorite,
      });
    },
    [activeGame, bulkFavorite],
  );

  const handleRenameRequest = useCallback((folder: ModFolder) => {
    setRenameDialog({ open: true, folder });
  }, []);

  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      if (!renameDialog.folder || !activeGame?.id) return;
      try {
        await renameMod.mutateAsync({
          folderPath: renameDialog.folder.path,
          newName,
          gameId: activeGame.id,
        });
        setRenameDialog({ open: false, folder: null });
        // After renaming, activePath in PreviewPanel will likely update via effectivePath
      } catch (err) {
        console.error('Rename failed', err);
      }
    },
    [renameDialog.folder, renameMod, activeGame?.id],
  );

  const handleRenameCancel = useCallback(() => {
    setRenameDialog({ open: false, folder: null });
  }, []);

  const handleDeleteRequest = useCallback((folder: ModFolder) => {
    setDeleteConfirm({ open: true, folder });
  }, []);

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteConfirm.folder) return;
    try {
      await deleteMod.mutateAsync({ path: deleteConfirm.folder.path });
      setDeleteConfirm({ open: false, folder: null });
      // The application will automatically deselect if the folder disappears from DB
    } catch (err) {
      console.error('Delete failed', err);
    }
  }, [deleteConfirm.folder, deleteMod]);

  const openMoveDialog = useCallback((folder: ModFolder) => {
    setMoveDialog({ open: true, folder });
  }, []);

  const closeMoveDialog = useCallback(() => {
    setMoveDialog({ open: false, folder: null });
  }, []);

  const handleMoveToObject = useCallback(
    async (
      folder: ModFolder,
      targetObjectId: string,
      status: 'disabled' | 'only-enable' | 'keep',
    ) => {
      if (!activeGame?.id) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('move_mod_to_object', {
          gameId: activeGame.id,
          folderPath: folder.path,
          targetObjectId,
          status,
        });
        updateFolderCache(queryClient, [folder.path], undefined, true);
        queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'none' });
      } catch (err) {
        console.error('Failed to move mod to object:', err);
      }
    },
    [queryClient, activeGame],
  );

  const handleToggleSafeRequest = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;

      // Block: active mods cannot change privacy context (Mutually Exclusive Corridor)
      if (folder.is_enabled) {
        toast.warning('Disable this mod before changing its privacy status.');
        return;
      }

      const safeMode = useAppStore.getState().safeMode;
      if (safeMode && folder.is_safe) {
        setPinSafeDialog({ open: true, folder });
      } else {
        toggleModSafe.mutate({
          gameId: activeGame.id,
          folderPath: folder.path,
          safe: !folder.is_safe,
        });
      }
    },
    [toggleModSafe, activeGame?.id],
  );

  const handleToggleSafeSubmit = useCallback(() => {
    if (!pinSafeDialog.folder || !activeGame?.id) return;
    toggleModSafe.mutate({
      gameId: activeGame.id,
      folderPath: pinSafeDialog.folder.path,
      safe: false,
    });
    setPinSafeDialog({ open: false, folder: null });
  }, [pinSafeDialog.folder, toggleModSafe, activeGame?.id]);

  const handleToggleSafeCancel = useCallback(() => {
    setPinSafeDialog({ open: false, folder: null });
  }, []);

  return {
    handleToggleEnabled,
    handleToggleFavorite,
    handleEnableOnlyThis,
    duplicateWarning,
    handleDuplicateForceEnable,
    handleDuplicateEnableOnly,
    handleDuplicateCancel,
    renameDialog,
    handleRenameRequest,
    handleRenameSubmit,
    handleRenameCancel,
    deleteConfirm,
    setDeleteConfirm,
    handleDeleteRequest,
    handleDeleteConfirm,
    moveDialog,
    openMoveDialog,
    closeMoveDialog,
    handleMoveToObject,
    pinSafeDialog,
    handleToggleSafeRequest,
    handleToggleSafeSubmit,
    handleToggleSafeCancel,
  };
}
