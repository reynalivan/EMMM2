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
  const activeGameId = activeGame?.id;
  const { mutate: bulkToggle } = useBulkToggle();
  const { mutate: bulkDelete } = useBulkDelete();
  const { mutate: bulkUpdateInfo } = useBulkUpdateInfo();
  const { mutate: bulkSafety } = useBulkSafety();
  const { mutate: bulkFavorite } = useBulkFavorite();
  const { mutate: bulkPin } = useBulkPin();

  const [bulkTagOpen, setBulkTagOpen] = useState(false);
  const [bulkDeleteConfirm, setBulkDeleteConfirm] = useState(false);

  // These are spread into every card's props; without useCallback they get a new
  // identity each render and defeat the React.memo on FolderCard/FolderListRow.
  const handleBulkToggle = useCallback(
    (enable: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGameId) return;
      bulkToggle({ gameId: activeGameId, paths, enable });
    },
    [activeGameId, bulkToggle, gridSelection],
  );

  const handleBulkTagRequest = useCallback(() => {
    setBulkTagOpen(true);
  }, []);

  // Bulk Add Tags — mutation toasts success/failure itself (see useBulkUpdateInfo)
  const handleBulkTagSubmit = useCallback(
    (tags: string[]) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGameId) return;
      bulkUpdateInfo({ gameId: activeGameId, paths, update: { tags_add: tags } });
    },
    [activeGameId, bulkUpdateInfo, gridSelection],
  );

  const handleBulkDeleteRequest = useCallback(() => {
    setBulkDeleteConfirm(true);
  }, []);

  const handleBulkDeleteConfirm = useCallback(() => {
    const paths = Array.from(gridSelection);
    if (paths.length === 0 || !activeGameId) return;
    bulkDelete(
      { paths, gameId: activeGameId },
      {
        onSuccess: () => {
          setBulkDeleteConfirm(false);
          clearGridSelection();
        },
      },
    );
  }, [activeGameId, bulkDelete, clearGridSelection, gridSelection]);

  // Bulk Favorite/Unfavorite — uses proper mutation hook with targeted cache
  const handleBulkFavorite = useCallback(
    (favorite: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGameId) return;
      bulkFavorite({ gameId: activeGameId, folderPaths: paths, favorite });
    },
    [activeGameId, bulkFavorite, gridSelection],
  );

  // Bulk Safe/Unsafe — uses existing bulk_update_info
  const handleBulkSafe = useCallback(
    (safe: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGameId) return;
      bulkSafety({ gameId: activeGameId, paths, safe });
    },
    [activeGameId, bulkSafety, gridSelection],
  );

  // Bulk Pin/Unpin — uses proper mutation hook with targeted cache
  const handleBulkPin = useCallback(
    (pin: boolean) => {
      const paths = Array.from(gridSelection);
      if (paths.length === 0 || !activeGameId) return;
      bulkPin({ gameId: activeGameId, folderPaths: paths, pin });
    },
    [activeGameId, bulkPin, gridSelection],
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
