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
} from '../../../hooks/useBulkModMutations';
import { useActiveGame } from '../../../hooks/useActiveGame';
import type { ModFolder } from '../../../types/object';

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

  // These are spread into every card's props; without useCallback they get a new
  // identity each render and defeat the React.memo on FolderCard/FolderListRow.
  const handleBulkToggle = useCallback(
    (enable: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkToggle.mutate({ gameId: activeGame.id, paths, enable });
    },
    [activeGame?.id, bulkToggle.mutate, gridSelection],
  );

  const handleBulkTagRequest = useCallback(() => {
    setBulkTagOpen(true);
  }, []);

  // Bulk Add Tags — mutation toasts success/failure itself (see useBulkUpdateInfo)
  const handleBulkTagSubmit = useCallback(
    (tags: string[]) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkUpdateInfo.mutate({ gameId: activeGame.id, paths, update: { tags_add: tags } });
    },
    [activeGame?.id, bulkUpdateInfo.mutate, gridSelection],
  );

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
  }, [bulkDelete.mutate, clearGridSelection, gridSelection]);

  // Bulk Favorite/Unfavorite — uses proper mutation hook with targeted cache
  const handleBulkFavorite = useCallback(
    (favorite: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkFavorite.mutate({ gameId: activeGame.id, folderPaths: paths, favorite });
    },
    [activeGame?.id, bulkFavorite.mutate, gridSelection],
  );

  // Bulk Safe/Unsafe — uses existing bulk_update_info
  const handleBulkSafe = useCallback(
    (safe: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkUpdateInfo.mutate({ gameId: activeGame.id, paths, update: { is_safe: safe } });
    },
    [activeGame?.id, bulkUpdateInfo.mutate, gridSelection],
  );

  // Bulk Pin/Unpin — uses proper mutation hook with targeted cache
  const handleBulkPin = useCallback(
    (pin: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkPin.mutate({ gameId: activeGame.id, folderPaths: paths, pin });
    },
    [activeGame?.id, bulkPin.mutate, gridSelection],
  );

  // Bulk Move to Object
  const handleBulkMoveToObject = useCallback(() => {
    const firstSelected = sortedFolders.find((f) => gridSelection.has(f.path));
    if (firstSelected) {
      openMoveDialog(firstSelected);
    }
  }, [gridSelection, openMoveDialog, sortedFolders]);

  return {
    bulkTagOpen,
    setBulkTagOpen,
    bulkDeleteConfirm,
    setBulkDeleteConfirm,
    handleBulkToggle,
    handleBulkTagRequest,
    handleBulkTagSubmit,
    handleBulkDeleteRequest,
    handleBulkDeleteConfirm,
    handleBulkFavorite,
    handleBulkSafe,
    handleBulkPin,
    handleBulkMoveToObject,
  };
}
