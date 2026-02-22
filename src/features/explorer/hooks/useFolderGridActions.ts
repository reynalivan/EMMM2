/**
 * useFolderGridActions — Single-item action handlers extracted from useFolderGrid.
 *
 * Handles: toggle enabled (with duplicate check), toggle favorite,
 * enable only this, rename, delete, move to object.
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
  folderKeys,
  ModFolder,
} from '../../../hooks/useFolders';
import type { DuplicateInfo } from '../../../types/mod';

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

  // Move To Object dialog state
  const [moveDialog, setMoveDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
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
        // Disabling — no duplicate check needed
        toggleMod.mutate({ path: folder.path, enable: false });
        return;
      }

      // Enabling — check for duplicates first
      if (!activeGame?.id) {
        toggleMod.mutate({ path: folder.path, enable: true });
        return;
      }

      try {
        const duplicates = await checkDuplicate.mutateAsync({
          folderPath: folder.path,
          gameId: activeGame.id,
        });

        if (duplicates.length > 0) {
          // Show warning modal
          setDuplicateWarning({ open: true, folder, duplicates });
        } else {
          // No duplicates — enable directly
          toggleMod.mutate({ path: folder.path, enable: true });
        }
      } catch {
        // Check failed — enable anyway
        toggleMod.mutate({ path: folder.path, enable: true });
      }
    },
    [toggleMod, activeGame, checkDuplicate],
  );

  // Duplicate Warning Modal handlers
  const handleDuplicateForceEnable = useCallback(() => {
    if (!duplicateWarning.folder) return;
    toggleMod.mutate({ path: duplicateWarning.folder.path, enable: true });
    setDuplicateWarning({ open: false, folder: null, duplicates: [] });
  }, [duplicateWarning.folder, toggleMod]);

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

  // Enable Only This — disable siblings, enable target
  const handleEnableOnlyThis = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;
      enableOnlyThis.mutate({ targetPath: folder.path, gameId: activeGame.id });
    },
    [activeGame, enableOnlyThis],
  );

  // Toggle Favorite
  const handleToggleFavorite = useCallback(
    async (folder: ModFolder) => {
      if (!folder.id) {
        console.warn('Cannot favorite folder without ID');
        return;
      }
      try {
        await import('@tauri-apps/api/core').then((m) =>
          m.invoke('toggle_favorite', {
            id: folder.id,
            favorite: !folder.is_favorite,
          }),
        );
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
      } catch (e) {
        console.error('Failed to toggle favorite:', e);
      }
    },
    [queryClient],
  );

  // Rename
  const handleRenameRequest = useCallback((folder: ModFolder) => {
    setRenamingId(folder.path);
  }, []);

  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      if (!renamingId) return;
      const folder = sortedFolders.find((f) => f.path === renamingId);
      if (!folder) return;

      try {
        await renameMod.mutateAsync({ folderPath: folder.path, newName });
        setRenamingId(null);
        clearGridSelection();
      } catch (err) {
        console.error('Rename failed', err);
      }
    },
    [renamingId, renameMod, sortedFolders, clearGridSelection],
  );

  const handleRenameCancel = useCallback(() => {
    setRenamingId(null);
  }, []);

  // Delete
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

  // Move mod to a different object — open/close dialog
  const openMoveDialog = useCallback((folder: ModFolder) => {
    setMoveDialog({ open: true, folder });
  }, []);

  const closeMoveDialog = useCallback(() => {
    setMoveDialog({ open: false, folder: null });
  }, []);

  // Move mod to a different object with status
  const handleMoveToObject = useCallback(
    async (
      folder: ModFolder,
      targetObjectId: string,
      status: 'disabled' | 'only-enable' | 'keep',
    ) => {
      if (!folder.id) {
        console.warn('Cannot move folder without DB ID');
        return;
      }
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('move_mod_to_object', {
          modId: folder.id,
          targetObjectId,
          status,
        });
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
      } catch (err) {
        console.error('Failed to move mod to object:', err);
      }
    },
    [queryClient],
  );

  return {
    // Toggle
    handleToggleEnabled,
    handleToggleFavorite,
    handleEnableOnlyThis,

    // Duplicate Warning
    duplicateWarning,
    handleDuplicateForceEnable,
    handleDuplicateEnableOnly,
    handleDuplicateCancel,

    // Rename
    renamingId,
    handleRenameRequest,
    handleRenameSubmit,
    handleRenameCancel,

    // Delete
    deleteConfirm,
    setDeleteConfirm,
    handleDeleteRequest,
    handleDeleteConfirm,

    // Move To Object
    moveDialog,
    openMoveDialog,
    closeMoveDialog,
    handleMoveToObject,
  };
}
