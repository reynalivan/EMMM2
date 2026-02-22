/**
 * useFolderGridBulk — Bulk action handlers extracted from useFolderGrid.
 *
 * Handles: bulk toggle, bulk delete, bulk tag, bulk favorite,
 * bulk safe, bulk pin, bulk move to object.
 */

import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  useBulkToggle,
  useBulkDelete,
  useBulkUpdateInfo,
  folderKeys,
  ModFolder,
} from '../../../hooks/useFolders';

interface FolderGridBulkOptions {
  gridSelection: Set<string>;
  sortedFolders: ModFolder[];
  clearGridSelection: () => void;
  openMoveDialog: (folder: ModFolder) => void;
}

export function useFolderGridBulk({
  gridSelection,
  sortedFolders,
  clearGridSelection,
  openMoveDialog,
}: FolderGridBulkOptions) {
  const queryClient = useQueryClient();
  const bulkToggle = useBulkToggle();
  const bulkDelete = useBulkDelete();
  const bulkUpdateInfo = useBulkUpdateInfo();

  const [bulkTagOpen, setBulkTagOpen] = useState(false);
  const [bulkDeleteConfirm, setBulkDeleteConfirm] = useState(false);

  const handleBulkToggle = useCallback(
    (enable: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0) return;
      bulkToggle.mutate({ paths, enable });
    },
    [gridSelection, bulkToggle],
  );

  const handleBulkTagRequest = useCallback(() => {
    setBulkTagOpen(true);
  }, []);

  const handleBulkDeleteRequest = useCallback(() => {
    setBulkDeleteConfirm(true);
  }, []);

  const handleBulkDeleteConfirm = useCallback(() => {
    const paths = Array.from(gridSelection);
    if (paths.length === 0) return;
    bulkDelete.mutate(
      { paths },
      {
        onSuccess: () => {
          setBulkDeleteConfirm(false);
          clearGridSelection();
        },
      },
    );
  }, [gridSelection, bulkDelete, clearGridSelection]);

  // Bulk Favorite/Unfavorite
  const handleBulkFavorite = useCallback(
    async (favorite: boolean) => {
      const ids = sortedFolders.filter((f) => gridSelection.has(f.path) && f.id).map((f) => f.id!);
      if (ids.length === 0) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('bulk_toggle_favorite', { ids, favorite });
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
      } catch (e) {
        console.error('Bulk favorite failed:', e);
      }
    },
    [gridSelection, sortedFolders, queryClient],
  );

  // Bulk Safe/Unsafe — uses existing bulk_update_info
  const handleBulkSafe = useCallback(
    (safe: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0) return;
      bulkUpdateInfo.mutate({ paths, update: { is_safe: safe } });
    },
    [gridSelection, bulkUpdateInfo],
  );

  // Bulk Pin/Unpin
  const handleBulkPin = useCallback(
    async (pin: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('bulk_pin_mods', { ids: paths, pin });
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
      } catch (e) {
        console.error('Bulk pin failed:', e);
      }
    },
    [gridSelection, queryClient],
  );

  // Bulk Move to Object
  const handleBulkMoveToObject = useCallback(() => {
    const firstSelected = sortedFolders.find((f) => gridSelection.has(f.path));
    if (firstSelected) {
      openMoveDialog(firstSelected);
    }
  }, [gridSelection, sortedFolders, openMoveDialog]);

  return {
    bulkTagOpen,
    setBulkTagOpen,
    bulkDeleteConfirm,
    setBulkDeleteConfirm,
    handleBulkToggle,
    handleBulkTagRequest,
    handleBulkDeleteRequest,
    handleBulkDeleteConfirm,
    handleBulkFavorite,
    handleBulkSafe,
    handleBulkPin,
    handleBulkMoveToObject,
  };
}
