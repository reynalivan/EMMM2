/**
 * useFolderGridActions — Single-item action handlers extracted from useFolderGrid.
 *
 * Handles: toggle enabled (with duplicate check), toggle favorite,
 * enable only this, rename, delete, move to object, toggle safe.
 */

import { useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import { join } from '@tauri-apps/api/path';
import { toast } from '../../../stores/useToastStore';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useSettings } from '../../../hooks/useSettings';
import {
  useToggleMod,
  useRenameMod,
  useDeleteMod,
  useEnableOnlyThis,
  useToggleModSafe,
  useBulkFavorite,
  updateFolderCache,
  ModFolder,
  folderKeys,
  useCheckDuplicate,
} from '../../../hooks/useFolders';
import { useAppStore } from '../../../stores/useAppStore';
import type { DuplicateInfo } from '../../../types/mod';
import type { ObjectSummary } from '../../../types/object';
import type { MatchedDbEntry } from '../../object-list/SyncConfirmModal';

interface FolderGridActionsOptions {
  sortedFolders: ModFolder[];
  objects: ObjectSummary[];
  clearGridSelection: () => void;
}

export function useFolderGridActions({
  sortedFolders,
  objects,
  clearGridSelection,
}: FolderGridActionsOptions) {
  const { t } = useTranslation(['grid', 'objects', 'common']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const { settings } = useSettings();
  const toggleMod = useToggleMod();
  const renameMod = useRenameMod();
  const deleteMod = useDeleteMod();
  const enableOnlyThis = useEnableOnlyThis();
  const toggleModSafe = useToggleModSafe();
  const bulkFavorite = useBulkFavorite();
  const checkDuplicate = useCheckDuplicate();

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

  const [duplicateWarning, setDuplicateWarning] = useState<{
    open: boolean;
    folder: ModFolder | null;
    duplicates: DuplicateInfo[];
  }>({ open: false, folder: null, duplicates: [] });

  // Sync with DB confirm state
  const [syncConfirm, setSyncConfirm] = useState<{
    open: boolean;
    folder: ModFolder | null;
    match: MatchedDbEntry | null;
    isLoading: boolean;
    currentData: {
      name: string;
      object_type: string;
      metadata: Record<string, unknown> | null;
      thumbnail_path: string | null;
    } | null;
  }>({
    open: false,
    folder: null,
    match: null,
    isLoading: false,
    currentData: null,
  });

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
    [toggleMod, activeGame?.id, checkDuplicate],
  );

  const handleDuplicateForceEnable = useCallback(() => {
    if (!duplicateWarning.folder || !activeGame?.id) return;
    toggleMod.mutate({ path: duplicateWarning.folder.path, enable: true, gameId: activeGame.id });
    setDuplicateWarning({ open: false, folder: null, duplicates: [] });
  }, [duplicateWarning.folder, toggleMod, activeGame?.id]);

  const handleDuplicateEnableOnly = useCallback(() => {
    if (!duplicateWarning.folder || !activeGame?.id) return;
    enableOnlyThis.mutate({
      targetPath: duplicateWarning.folder.path,
      gameId: activeGame.id,
    });
    setDuplicateWarning({ open: false, folder: null, duplicates: [] });
  }, [duplicateWarning.folder, activeGame?.id, enableOnlyThis]);

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

      // AC-13.2.2: Pre-flight check for Windows-illegal characters.
      // Backend also validates, but this avoids the IPC roundtrip and gives instant feedback.
      const ILLEGAL_CHARS = /[\\/:*?"<>|]/;
      if (ILLEGAL_CHARS.test(newName)) {
        toast.error(t('objects:edit_modal.validation.path_invalid'));
        return;
      }

      try {
        await renameMod.mutateAsync({ folderPath: folder.path, newName, gameId: activeGame.id });
        setRenamingId(null);
        clearGridSelection();
      } catch (err) {
        console.error('Rename failed', err);
      }
    },
    [renamingId, renameMod, sortedFolders, clearGridSelection, activeGame?.id, t],
  );

  const handleRenameCancel = useCallback(() => {
    setRenamingId(null);
  }, []);

  const handleDeleteRequest = useCallback(async (folder: ModFolder) => {
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
  const isTogglingObjectRef = useRef(false);

  const toggleObjectMods = useCallback(
    async (objectId: string, enable: boolean) => {
      if (!activeGame || isTogglingObjectRef.current) return;

      const obj = objects.find((o) => o.id === objectId);
      if (!obj) return;

      isTogglingObjectRef.current = true;
      try {
        const targetPath = await join(activeGame.mod_path, obj.folder_path);
        await toggleMod.mutateAsync({
          path: targetPath,
          enable,
          gameId: activeGame.id,
        });
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
      } catch (err) {
        console.error('Failed to toggle object mods:', err);
      } finally {
        isTogglingObjectRef.current = false;
      }
    },
    [activeGame, objects, queryClient, toggleMod],
  );

  const handleRevealInExplorer = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      const obj = objects.find((o) => o.id === objectId);
      try {
        await commands.revealObjectInExplorer({
          gameId: activeGame.id,
          objectId,
          objectName: obj?.folder_path ?? objectId,
        });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(msg);
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      }
    },
    [activeGame, objects, queryClient],
  );

  // ── Sync with DB ────────────────────────────────────────────────
  const handleSyncWithDb = useCallback(
    async (folder: ModFolder) => {
      if (!activeGame?.id) return;
      setSyncConfirm({
        open: true,
        folder,
        match: null,
        isLoading: true,
        currentData: {
          name: folder.name,
          object_type: folder.category ?? '',
          metadata: folder.metadata ?? null,
          thumbnail_path: folder.thumbnail_path,
        },
      });
      try {
        const match = await commands.matchObjectWithDb({
          gameType: activeGame.game_type,
          objectName: folder.name,
        });
        setSyncConfirm((prev) => ({ ...prev, match: match as MatchedDbEntry, isLoading: false }));
      } catch (e) {
        console.error('matchObjectWithDb failed:', e);
        setSyncConfirm((prev) => ({ ...prev, isLoading: false }));
      }
    },
    [activeGame?.id, activeGame?.game_type],
  );

  const handleCloseSyncConfirm = useCallback(() => {
    setSyncConfirm({ open: false, folder: null, match: null, isLoading: false, currentData: null });
  }, []);

  const handleApplySyncMatch = useCallback(
    async (match: MatchedDbEntry) => {
      const { folder } = syncConfirm;
      if (!folder || !activeGame?.id) return;
      try {
        let currentPath = folder.path;
        // Rename if name changed
        if (match.name && match.name !== folder.name) {
          const result = await commands.renameModFolder({
            folderPath: folder.path,
            newName: match.name,
            gameId: activeGame.id,
          });
          currentPath = result.new_path;
        }
        // Apply category if changed
        if (match.object_type) {
          await commands.setModCategory({
            gameId: activeGame.id,
            folderPath: currentPath,
            category: match.object_type,
          });
        }
        // Apply metadata if present
        if (match.metadata) {
          const metaStrings: Record<string, string> = {};
          Object.entries(match.metadata).forEach(([k, v]) => {
            if (v !== undefined && v !== null) metaStrings[k] = String(v);
          });
          await commands.updateModInfo({
            folderPath: currentPath,
            update: { metadata: metaStrings },
          });
        }
        // Apply thumbnail if present
        if (match.thumbnail_path) {
          await commands.updateModThumbnail({
            folderPath: currentPath,
            sourcePath: match.thumbnail_path,
          });
        }
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        toast.success(t('objects:edit_modal.success_message', { name: folder.name }));
        setSyncConfirm({
          open: false,
          folder: null,
          match: null,
          isLoading: false,
          currentData: null,
        });
      } catch (e) {
        console.error('Apply sync match failed:', e);
        toast.error(t('objects:edit_modal.error_message', { error: String(e) }));
      }
    },
    [syncConfirm, activeGame?.id, queryClient, t],
  );

  const handleMoveToObject = useCallback(
    async (
      folder: ModFolder,
      targetObjectId: string,
      status: 'disabled' | 'only-enable' | 'keep',
    ) => {
      if (!activeGame?.id) return;
      try {
        await commands.moveModToObject({
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

  const handleToggleSafeRequest = useCallback(
    (folder: ModFolder) => {
      if (!activeGame?.id) return;

      // Phase 24/25 barrier: Active mods cannot switch privacy context
      if (folder.is_enabled) {
        setActiveContextDialog({ open: true, folder, isProcessing: false });
        return;
      }

      const safeMode = useAppStore.getState().safeMode;
      const hasPin = !!settings?.safe_mode?.pin_hash;
      // Only prompt for PIN when: (1) Safe Mode is ON globally, (2) marking as Unsafe, (3) PIN is configured
      if (safeMode && folder.is_safe && hasPin) {
        setPinSafeDialog({ open: true, folder });
      } else {
        // No PIN configured, or marking as Safe — allow direct toggle
        toggleModSafe.mutate({
          gameId: activeGame.id,
          folderPath: folder.path,
          safe: !folder.is_safe,
        });
      }
    },
    [toggleModSafe, activeGame?.id, settings],
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

      // 1. Disable the active mod
      const newPath = await commands.toggleMod({
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
      const hasPin = !!settings?.safe_mode?.pin_hash;
      const targetSafeStatus = !folder.is_safe;

      // 2. Check if we are jumping to Unsafe while SafeMode is globally ON, and a PIN is configured
      if (safeMode && targetSafeStatus === false && hasPin) {
        // Requires PIN. Hand off to pinSafeDialog with the newly disabled mod.
        setActiveContextDialog({ open: false, folder: null, isProcessing: false });
        setPinSafeDialog({
          open: true,
          folder: { ...folder, path: newPath, is_enabled: false },
        });
        return;
      }

      // 3. Perform the privacy context switch directly (no PIN needed)
      await commands.toggleModSafe({
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

      toast.success(
        t('objects:toasts.mark_safe_context', {
          context: targetSafeStatus ? t('common:contexts.safe') : t('common:contexts.unsafe'),
        }),
      );
    } catch (err) {
      toast.error(t('objects:create_modal.error_message', { error: String(err) }));
    } finally {
      setActiveContextDialog({ open: false, folder: null, isProcessing: false });

      // Invalidate everything to be absolutely sure the UI maps the disk
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['objects'], refetchType: 'active' });
    }
  }, [activeContextDialog.folder, activeGame?.id, queryClient, settings, t]);

  return {
    handleToggleEnabled,
    handleToggleFavorite,
    handleEnableOnlyThis,
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
    duplicateWarning,
    handleDuplicateForceEnable,
    handleDuplicateEnableOnly,
    handleDuplicateCancel,
    toggleObjectMods,
    handleRevealInExplorer,
    /** Sync with DB */
    syncConfirm,
    handleSyncWithDb,
    handleCloseSyncConfirm,
    handleApplySyncMatch,
    /** true when a PIN has been configured in settings */
    hasPin: !!settings?.safe_mode?.pin_hash,
  };
}
