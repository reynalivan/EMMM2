import { useRef, useMemo, useState, useCallback, useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useAppStore } from '../../../stores/useAppStore';
import { useResponsive } from '../../../hooks/useResponsive';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useObjects } from '../../../hooks/useObjects';
import {
  useModFolders,
  sortFolders,
  folderKeys,
  useToggleMod,
  useRenameMod,
  useDeleteMod,
  useBulkToggle,
  useBulkDelete,
  useBulkUpdateInfo,
  useEnableOnlyThis,
  useCheckDuplicate,
  ModFolder,
} from '../../../hooks/useFolders';
import type { DuplicateInfo } from '../../../types/mod';
import { useFolderNavigation } from '../../../hooks/useFolderNavigation';
import { useFileDrop } from '../../../hooks/useFileDrop';

// Grid layout constants
const CARD_MIN_W = 160;
const CARD_MAX_W = 280;
const CARD_INFO_H = 70;
const LIST_ROW_HEIGHT = 52;
const GAP = 12;

export function useFolderGrid() {
  const {
    selectedObject,
    currentPath,
    setCurrentPath,
    gridSelection,
    toggleGridSelection,
    clearGridSelection,
    setMobilePane,
    sortField,
    sortOrder,
    setSortField,
    setSortOrder,
    viewMode,
    setViewMode,
    safeMode,
    explorerSearchQuery,
    setExplorerSearch,
    explorerSubPath,
    setExplorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
  } = useAppStore();

  const { isMobile } = useResponsive();
  const parentRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(800);

  // Move To Object dialog state
  const [moveDialog, setMoveDialog] = useState<{ open: boolean; folder: ModFolder | null }>({
    open: false,
    folder: null,
  });

  // Fetch real data from backend — when an object is selected, filter by object_id (DB),
  // not by sub_path (filesystem). Objects are virtual categories, not physical folders.
  const queryClient = useQueryClient();
  const {
    data: rawFolders = [],
    isLoading,
    isError,
    error,
  } = useModFolders(explorerSubPath, selectedObject ?? undefined);
  const toggleMod = useToggleMod();
  const renameMod = useRenameMod();
  const deleteMod = useDeleteMod();
  const bulkToggle = useBulkToggle();
  const bulkDelete = useBulkDelete();
  const bulkUpdateInfo = useBulkUpdateInfo();
  const { activeGame } = useActiveGame();

  const enableOnlyThis = useEnableOnlyThis();

  // Object Mode: fetch objects so we can navigate grid when sidebar object is clicked
  const { data: objects = [] } = useObjects();

  // When an object is selected from the sidebar, filter mods by object_id (DB query).
  // Objects are virtual DB categories — NOT physical subfolders on disk.
  const prevSelectedRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedObject || selectedObject === prevSelectedRef.current) return;
    prevSelectedRef.current = selectedObject;

    const obj = objects.find((o) => o.id === selectedObject);
    if (obj) {
      // Navigate into the object's physical folder under mod_path
      // instead of resetting to the root
      setExplorerSubPath(obj.name);
      setCurrentPath([obj.name]);
      clearGridSelection();
    }
  }, [selectedObject, objects, setExplorerSubPath, setCurrentPath, clearGridSelection]);

  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    open: boolean;
    folder: ModFolder | null;
  }>({ open: false, folder: null });
  const [bulkTagOpen, setBulkTagOpen] = useState(false);
  const [bulkDeleteConfirm, setBulkDeleteConfirm] = useState(false);
  const checkDuplicate = useCheckDuplicate();
  const [duplicateWarning, setDuplicateWarning] = useState<{
    open: boolean;
    folder: ModFolder | null;
    duplicates: DuplicateInfo[];
  }>({ open: false, folder: null, duplicates: [] });

  // Filter by search query
  const filteredFolders = useMemo(() => {
    const safeModeFiltered = safeMode ? rawFolders.filter((folder) => folder.is_safe) : rawFolders;
    if (!explorerSearchQuery) return safeModeFiltered;
    const q = explorerSearchQuery.toLowerCase();
    return safeModeFiltered.filter((folder) => folder.name.toLowerCase().includes(q));
  }, [rawFolders, safeMode, explorerSearchQuery]);

  // Sort folders
  const sortedFolders = useMemo(
    () => sortFolders(filteredFolders, sortField, sortOrder),
    [filteredFolders, sortField, sortOrder],
  );

  // Determine layout: grid vs list
  const isGridView = viewMode === 'grid' && !isMobile;
  const columnCount = isGridView
    ? Math.max(1, Math.floor((containerWidth + GAP) / (CARD_MIN_W + GAP)))
    : 1;
  const cardWidth = isGridView
    ? Math.min(CARD_MAX_W, Math.floor((containerWidth - GAP * (columnCount - 1)) / columnCount))
    : 0;
  const cardHeight = isGridView ? Math.round(cardWidth * (4 / 3)) + CARD_INFO_H : 0;
  const rowCount = isGridView
    ? Math.ceil(sortedFolders.length / columnCount)
    : sortedFolders.length;

  // Track container width
  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // TanStack Virtual
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => (isGridView ? cardHeight + GAP : LIST_ROW_HEIGHT),
    overscan: 5,
    initialOffset: explorerScrollOffset,
  });

  // Reset scroll to top when navigating to a new folder
  useEffect(() => {
    rowVirtualizer.scrollToOffset(0);
  }, [explorerSubPath]); // eslint-disable-line react-hooks/exhaustive-deps

  // Persist scroll offset on scroll (debounced via RAF)
  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    let rafId: number;
    const handleScroll = () => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        setExplorerScrollOffset(el.scrollTop);
      });
    };
    el.addEventListener('scroll', handleScroll, { passive: true });
    return () => {
      el.removeEventListener('scroll', handleScroll);
      cancelAnimationFrame(rafId);
    };
  }, [setExplorerScrollOffset]);

  // Navigation handlers
  const handleNavigate = useCallback(
    (folderName: string) => {
      const newPath = [...currentPath, folderName];
      setCurrentPath(newPath);
      // When an object is selected, currentPath[0] is the object display name (not a FS folder).
      // Strip it when building the filesystem sub-path.
      const fsPath = selectedObject ? newPath.slice(1) : newPath;
      setExplorerSubPath(fsPath.length > 0 ? fsPath.join('/') : undefined);
      setExplorerScrollOffset(0);
      clearGridSelection();
    },
    [
      currentPath,
      setCurrentPath,
      setExplorerSubPath,
      setExplorerScrollOffset,
      clearGridSelection,
      selectedObject,
    ],
  );

  const handleBreadcrumbClick = useCallback(
    (index: number) => {
      // If object is selected, we cannot navigate ABOVE the object level (index 0).
      // The Object Name is always at index 0 in this mode.
      if (selectedObject && index < 0) return;

      const newPath = currentPath.slice(0, index + 1);
      setCurrentPath(newPath);
      // When an object is selected, currentPath[0] is the object display name (not a FS folder).
      // Strip it when building the filesystem sub-path.
      const fsPath = selectedObject ? newPath.slice(1) : newPath;
      setExplorerSubPath(fsPath.length > 0 ? fsPath.join('/') : undefined);
      setExplorerScrollOffset(0);
      clearGridSelection();
    },
    [
      currentPath,
      setCurrentPath,
      setExplorerSubPath,
      setExplorerScrollOffset,
      clearGridSelection,
      selectedObject,
    ],
  );

  const handleGoHome = useCallback(() => {
    // If an object is selected, "Home" means the object's root view (DB-filtered, no FS sub-path).
    if (selectedObject) {
      const obj = objects.find((o) => o.id === selectedObject);
      if (obj) {
        setCurrentPath([obj.name]);
        setExplorerSubPath(undefined);
        setExplorerScrollOffset(0);
        clearGridSelection();
        return;
      }
    }

    // Default behavior (No object selected) - Go to Game Root
    setCurrentPath([]);
    setExplorerSubPath(undefined);
    setExplorerScrollOffset(0);
    clearGridSelection();
  }, [
    setCurrentPath,
    setExplorerSubPath,
    setExplorerScrollOffset,
    clearGridSelection,
    selectedObject,
    objects,
  ]);

  const handleSortToggle = useCallback(() => {
    const fields = ['name', 'modified_at', 'size_bytes'] as const;
    const currentIdx = fields.indexOf(sortField);
    if (sortOrder === 'desc') {
      const nextIdx = (currentIdx + 1) % fields.length;
      setSortField(fields[nextIdx]);
      setSortOrder('asc');
    } else {
      setSortOrder('desc');
    }
  }, [sortField, sortOrder, setSortField, setSortOrder]);

  const sortLabel = sortField === 'name' ? 'Name' : sortField === 'modified_at' ? 'Date' : 'Size';

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
      // Use the new Atomic Command (DB + JSON)
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
        // Invalidate queries to refresh UI
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

  // Bulk Actions
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
    // Use the first selected folder as the reference for the dialog
    const firstSelected = sortedFolders.find((f) => gridSelection.has(f.path));
    if (firstSelected) {
      setMoveDialog({ open: true, folder: firstSelected });
    }
  }, [gridSelection, sortedFolders]);

  // Keyboard navigation
  const { focusedId, handleKeyDown } = useFolderNavigation({
    items: sortedFolders,
    gridColumns: isGridView ? columnCount : 1,
    getId: (item) => item.path,
    onNavigate: (item) => handleNavigate(item.folder_name),
    onSelectionChange: (item, multi) => toggleGridSelection(item.path, multi),
    onSelectAll: () => {
      // Select all visible items
      sortedFolders.forEach((f) => toggleGridSelection(f.path, true));
    },
    onDelete: (items) => {
      if (items.length > 0) handleDeleteRequest(items[0]);
    },
    onRename: (item) => handleRenameRequest(item),
    onGoUp: () => {
      if (currentPath.length > 0) {
        handleBreadcrumbClick(currentPath.length - 2);
      }
    },
  });

  // Drag & Drop — visual feedback only; actual import handled by MainLayout's SmartDropModal
  const noopDrop = useCallback((_paths: string[]) => {}, []);
  const { isDragging } = useFileDrop({ onDrop: noopDrop });

  // Manual Refresh — invalidate cache + trigger sync
  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: folderKeys.all });
  }, [queryClient]);

  // Move mod to a different object
  // Open Move To Object dialog
  const openMoveDialog = useCallback((folder: ModFolder) => {
    setMoveDialog({ open: true, folder });
  }, []);

  // Close dialog
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
        // Optionally, update enabled state if needed (optimistic UI)
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
      } catch (err) {
        console.error('Failed to move mod to object:', err);
      }
    },
    [queryClient],
  );

  return {
    moveDialog,
    openMoveDialog,
    closeMoveDialog,
    // State
    rawFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    isGridView,
    isMobile,
    selectedObject, // Expose selectedObject
    currentPath,
    explorerSearchQuery,
    sortField,
    sortOrder,
    sortLabel,
    viewMode,

    // Virtualization
    parentRef,
    rowVirtualizer,
    columnCount,
    cardWidth,

    // Navigation
    handleNavigate,
    handleBreadcrumbClick,
    handleGoHome,
    setMobilePane,
    setViewMode,
    setExplorerSearch,
    handleSortToggle,
    handleKeyDown,
    focusedId,
    handleRefresh,

    // Selection
    gridSelection,
    toggleGridSelection,
    clearGridSelection,

    // Actions
    handleToggleEnabled,
    handleToggleFavorite,
    handleEnableOnlyThis,
    handleMoveToObject,
    objects,

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

    // Bulk
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

    // DnD
    isDragging,
  };
}
