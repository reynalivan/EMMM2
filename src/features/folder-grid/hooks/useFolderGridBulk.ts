/**
 * useFolderGridBulk — Bulk action handlers extracted from useFolderGrid.
 *
 * Handles: bulk toggle, bulk delete, bulk tag, bulk favorite,
 * bulk safe, bulk pin, bulk move to object.
 */

import { useState, useCallback } from 'react';
import {
  useBulkToggle,
  useBulkDelete,
  useBulkUpdateInfo,
  useBulkFavorite,
  useBulkPin,
  ModFolder,
} from '../../../hooks/useFolders';
import { useActiveGame } from '../../../hooks/useActiveGame';

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
  const { activeGame } = useActiveGame();
  const bulkToggle = useBulkToggle();
  const bulkDelete = useBulkDelete();
  const bulkUpdateInfo = useBulkUpdateInfo();
  const bulkFavorite = useBulkFavorite();
  const bulkPin = useBulkPin();

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

  // Bulk Favorite/Unfavorite — uses proper mutation hook with targeted cache
  const handleBulkFavorite = useCallback(
    (favorite: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkFavorite.mutate({ gameId: activeGame.id, folderPaths: paths, favorite });
    },
    [gridSelection, activeGame, bulkFavorite],
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

  // Bulk Pin/Unpin — uses proper mutation hook with targeted cache
  const handleBulkPin = useCallback(
    (pin: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkPin.mutate({ gameId: activeGame.id, folderPaths: paths, pin });
    },
    [gridSelection, activeGame, bulkPin],
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
