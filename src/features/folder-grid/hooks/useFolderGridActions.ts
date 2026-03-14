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
  useBulkFavorite,
  updateFolderCache,
  ModFolder,
  folderKeys,
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
  const bulkFavorite = useBulkFavorite();

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

  const [activeContextDialog, setActiveContextDialog] = useState<{
    open: boolean;
    folder: ModFolder | null;
    isProcessing: boolean;
  }>({
    open: false,
    folder: null,
    isProcessing: false,
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
        // Duplicate check failed — proceed with enable anyway
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
    setRenamingId(folder.path);
  }, []);

  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      if (!renamingId || !activeGame?.id) return;
      const folder = sortedFolders.find((f) => f.path === renamingId);
      if (!folder) return;

      try {
        await renameMod.mutateAsync({ folderPath: folder.path, newName, gameId: activeGame.id });
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
        // Targeted: remove moved folder from current view
        updateFolderCache(queryClient, [folder.path], undefined, true);
        queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'none' });
      } catch (err) {
        console.error('Failed to move mod to object:', err);
      }
    },
    [queryClient, activeGame],
  );

  // Toggle Safe Mode per-mod
  const handleToggleSafeRequest = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;

      // Phase 24/25 barrier: Active mods cannot switch privacy context
      if (folder.is_enabled) {
        setActiveContextDialog({ open: true, folder, isProcessing: false });
        return;
      }

      const safeMode = useAppStore.getState().safeMode;
      // If Safe Mode is ON globally, configuring a mod as Unsafe (is_safe -> false) requires PIN
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

  const handleActiveContextCancel = useCallback(() => {
    setActiveContextDialog({ open: false, folder: null, isProcessing: false });
  }, []);

  const handleActiveContextSubmit = useCallback(async () => {
    if (!activeContextDialog.folder || !activeGame?.id) return;
    const folder = activeContextDialog.folder;

    try {
      setActiveContextDialog((prev) => ({ ...prev, isProcessing: true }));
      const { invoke } = await import('@tauri-apps/api/core');

      // 1. Disable the active mod
      const newPath = await invoke<string>('toggle_mod', {
        path: folder.path,
        enable: false,
        gameId: activeGame.id,
      });

      // Optimistically update the cache to show it's disabled temporarily
      updateFolderCache(queryClient, [folder.path], (f) => ({
        ...f,
        path: newPath,
        is_enabled: false,
      }));

      const safeMode = useAppStore.getState().safeMode;
      const targetSafeStatus = !folder.is_safe;

      // 2. Check if we are jumping to Unsafe while SafeMode is globally ON
      if (safeMode && targetSafeStatus === false) {
        // Requires PIN. Hand off to pinSafeDialog with the newly disabled mod.
        setActiveContextDialog({ open: false, folder: null, isProcessing: false });
        setPinSafeDialog({
          open: true,
          folder: { ...folder, path: newPath, is_enabled: false },
        });
        return;
      }

      // 3. Perform the privacy context switch directly (no PIN needed)
      await invoke<void>('toggle_mod_safe', {
        gameId: activeGame.id,
        folderPath: newPath,
        safe: targetSafeStatus,
      });

      // 4. Aggressive cache removal (it left the mutually exclusive corridor)
      updateFolderCache(queryClient, [folder.path, newPath], undefined, true);
      const appStore = useAppStore.getState();
      if (appStore.gridSelection?.has(folder.path) || appStore.gridSelection?.has(newPath)) {
        appStore.clearGridSelection();
      }

      import('../../../stores/useToastStore').then(({ toast }) => {
        toast.success(`Mod disabled and moved to ${targetSafeStatus ? 'Safe' : 'Unsafe'} context.`);
      });
    } catch (err) {
      import('../../../stores/useToastStore').then(({ toast }) => {
        toast.error(`Failed to switch context: ${String(err)}`);
      });
    } finally {
      setActiveContextDialog({ open: false, folder: null, isProcessing: false });

      // Invalidate everything to be absolutely sure the UI maps the disk
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'active' });
    }
  }, [activeContextDialog.folder, activeGame?.id, queryClient]);

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
    activeContextDialog,
    handleActiveContextCancel,
    handleActiveContextSubmit,
  };
}
