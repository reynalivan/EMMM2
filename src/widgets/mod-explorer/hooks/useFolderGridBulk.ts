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
  useBulkSafety,
  useBulkFavorite,
  useBulkPin,
} from '@/features/mod-runtime';
import { useActiveGame } from '@/entities/game';
import type { ModFolder } from '@/entities/game-object';

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
  const bulkSafety = useBulkSafety();
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
    [activeGame, bulkToggle, gridSelection],
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
    [activeGame, bulkUpdateInfo, gridSelection],
  );

  const handleBulkDeleteRequest = useCallback(() => {
    setBulkDeleteConfirm(true);
  }, []);

  const handleBulkDeleteConfirm = useCallback(() => {
    const paths = Array.from(gridSelection);
    if (paths.length === 0 || !activeGame?.id) return;
    bulkDelete.mutate(
      { paths, gameId: activeGame.id },
      {
        onSuccess: () => {
          setBulkDeleteConfirm(false);
          clearGridSelection();
        },
      },
    );
  }, [activeGame, bulkDelete, clearGridSelection, gridSelection]);

  // Bulk Favorite/Unfavorite — uses proper mutation hook with targeted cache
  const handleBulkFavorite = useCallback(
    (favorite: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkFavorite.mutate({ gameId: activeGame.id, folderPaths: paths, favorite });
    },
    [activeGame, bulkFavorite, gridSelection],
  );

  // Bulk Safe/Unsafe — uses existing bulk_update_info
  const handleBulkSafe = useCallback(
    (safe: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkSafety.mutate({ gameId: activeGame.id, paths, safe });
    },
    [activeGame, bulkSafety, gridSelection],
  );

  // Bulk Pin/Unpin — uses proper mutation hook with targeted cache
  const handleBulkPin = useCallback(
    (pin: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGame?.id) return;
      bulkPin.mutate({ gameId: activeGame.id, folderPaths: paths, pin });
    },
    [activeGame, bulkPin, gridSelection],
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
