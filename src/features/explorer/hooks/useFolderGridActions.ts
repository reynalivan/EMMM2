/**
 * useFolderGridActions — Single-item action handlers extracted from useFolderGrid.
 *
 * Handles: toggle enabled (with duplicate check), toggle favorite,
 * enable only this, rename, delete, move to object, toggle safe.
 */

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
  folderKeys,
  ModFolder,
} from '../../../hooks/useFolders';
import type { DuplicateInfo } from '../../../types/mod';
import { useAppStore } from '../../../stores/useAppStore';

interface FolderGridActionsOptions {
  sortedFolders: ModFolder[];
  clearGridSelection: () => void;
}

export function useFolderGridActions({
  sortedFolders,
  clearGridSelection,
}: FolderGridActionsOptions) {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const toggleMod = useToggleMod();
  const renameMod = useRenameMod();
  const deleteMod = useDeleteMod();
  const enableOnlyThis = useEnableOnlyThis();
  const checkDuplicate = useCheckDuplicate();
  const toggleModSafe = useToggleModSafe();

  // Move To Object dialog state
  const [moveDialog, setMoveDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
    open: false,
    folder: null,
  });

  // Pin / Safe mode dialog state
  const [pinSafeDialog, setPinSafeDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
    open: false,
    folder: null,
  });

  // Rename state
  const [renamingId, setRenamingId] = useState<string | null>(null);

  // Delete confirmation state
  const [deleteConfirm, setDeleteConfirm] = useState<{
    open: boolean;
    folder: ModFolder | null;
  }>({ open: false, folder: null });

  // Duplicate warning state
  const [duplicateWarning, setDuplicateWarning] = useState<{
    open: boolean;
    folder: ModFolder | null;
    duplicates: DuplicateInfo[];
  }>({ open: false, folder: null, duplicates: [] });

  // Toggle enabled — check for duplicates when enabling
  const handleToggleEnabled = useCallback(
    async (folder: ModFolder) => {
      if (folder.is_enabled) {
        if (activeGame?.id) {
          toggleMod.mutate({ path: folder.path, enable: false, gameId: activeGame.id });
        }
        return;
      }

      if (!activeGame?.id) {
        if (activeGame?.id) {
          toggleMod.mutate({ path: folder.path, enable: true, gameId: activeGame.id });
        }
        return;
      }

      try {
        const duplicates = await checkDuplicate.mutateAsync({
          folderPath: folder.path,
          gameId: activeGame.id,
        });

        if (duplicates.length > 0) {
          setDuplicateWarning({ open: true, folder, duplicates });
        } else {
          if (activeGame?.id) {
            toggleMod.mutate({ path: folder.path, enable: true, gameId: activeGame.id });
          }
        }
      } catch {
        if (activeGame?.id) {
          toggleMod.mutate({ path: folder.path, enable: true, gameId: activeGame.id });
        }
      }
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
    async (folder: ModFolder) => {
      if (!activeGame?.id) return;
      try {
        await import('@tauri-apps/api/core').then((m) =>
          m.invoke('toggle_favorite', {
            gameId: activeGame.id,
            folderPath: folder.path,
            favorite: !folder.is_favorite,
          }),
        );
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
      } catch (e) {
        console.error('Failed to toggle favorite:', e);
      }
    },
    [queryClient, activeGame?.id],
  );

  const handleRenameRequest = useCallback((folder: ModFolder) => {
    setRenamingId(folder.path);
  }, []);

  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      if (!renamingId) return;
      const folder = sortedFolders.find((f) => f.path === renamingId);
      if (!folder) return;

      try {
        if (activeGame?.id) {
          await renameMod.mutateAsync({ folderPath: folder.path, newName, gameId: activeGame.id });
        }
        setRenamingId(null);
        clearGridSelection();
      } catch (err) {
        console.error('Rename failed', err);
      }
    },
    [renamingId, renameMod, sortedFolders, clearGridSelection, activeGame?.id],
  );

  const handleRenameCancel = useCallback(() => {
    setRenamingId(null);
  }, []);

  const handleDeleteRequest = useCallback((folder: ModFolder) => {
    setDeleteConfirm({ open: true, folder });
  }, []);

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteConfirm.folder) return;
    try {
      await deleteMod.mutateAsync({ path: deleteConfirm.folder.path });
      setDeleteConfirm({ open: false, folder: null });
      clearGridSelection();
    } catch (err) {
      console.error('Delete failed', err);
    }
  }, [deleteConfirm.folder, deleteMod, clearGridSelection]);

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
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
      } catch (err) {
        console.error('Failed to move mod to object:', err);
      }
    },
    [queryClient, activeGame?.id],
  );

  // Toggle Safe Mode per-mod
  const handleToggleSafeRequest = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;

      const safeMode = useAppStore.getState().safeMode;
      // If Safe Mode is ON globally, configuring a mod as NSFW (is_safe -> false) requires PIN
      if (safeMode && folder.is_safe) {
        setPinSafeDialog({ open: true, folder });
      } else {
        // Otherwise, allow direct toggle without pin
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
    renamingId,
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
