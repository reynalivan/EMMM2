/**
 * useFolderGrid — Orchestrator hook for the FolderGrid component.
 *
 * Composes: useFolderGridNav, useFolderGridActions, useFolderGridBulk,
 *           useFolderGridLayout, useFolderGridImport
 * and manages data fetching, keyboard, and selection.
 */

import { useRef, useMemo, useState, useCallback, useEffect } from 'react';
import { join } from '@tauri-apps/api/path';
import { useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../../stores/useAppStore';
import { useResponsive } from '../../../hooks/useResponsive';
import { useObjects } from '../../../hooks/useObjects';
import { useModFolders, sortFolders, useToggleMod, ModFolder } from '../../../hooks/useFolders';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useFolderNavigation } from './useFolderNavigation';
import { useFolderGridNav } from './useFolderGridNav';
import { useFolderGridActions } from './useFolderGridActions';
import { useFolderGridBulk } from './useFolderGridBulk';
import { useFolderGridLayout } from './useFolderGridLayout';
import { useFolderGridImport } from './useFolderGridImport';
import { syncExplorerAfterRename } from '../../object-list/objHandlersHelpers';

export function useFolderGrid() {
  const {
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
    explorerSearchQuery,
    setExplorerSearch,
    explorerSubPath,
    setExplorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    isPreviewOpen,
    togglePreview,
    setGridSelection,
  } = useAppStore();

  const { isMobile } = useResponsive();
  const parentRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  // ── Data Fetching ─────────────────────────────────────────────────────────
  const {
    data: rawResponse,
    isLoading,
    isError,
    error,
    isPlaceholderData,
  } = useModFolders(explorerSubPath);

  const rawFolders = useMemo(
    () => rawResponse?.children || ([] as ModFolder[]),
    [rawResponse?.children],
  );
  const selfNodeType = rawResponse?.self_node_type || null;
  const selfIsMod = rawResponse?.self_is_mod ?? false;
  const selfIsEnabled = rawResponse?.self_is_enabled ?? false;
  const selfReasons = rawResponse?.self_classification_reasons || [];
  const conflicts = rawResponse?.conflicts || [];
  // Display name of the nearest disabled ancestor in the current sub_path.
  // Null means the current location is not locked by any parent.
  const ancestorDisabledBy = rawResponse?.ancestor_disabled_by ?? null;
  const ancestorDisabledPath = rawResponse?.ancestor_disabled_path ?? null;

  // Dialog state — "Enable Parent" confirmation
  const [enableParentDialogOpen, setEnableParentDialogOpen] = useState(false);

  const { data: objects = [] } = useObjects();

  // Sync store's selectedObjectFolderPath → explorerSubPath (decoupled from ObjectList query)
  const { selectedObjectFolderPath } = useAppStore();
  const prevSyncKeyRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedObjectFolderPath) return;

    if (selectedObjectFolderPath === prevSyncKeyRef.current) return;

    prevSyncKeyRef.current = selectedObjectFolderPath;

    setExplorerSubPath(selectedObjectFolderPath);
    // Derive breadcrumb name from folder path's last segment
    const displayName = selectedObjectFolderPath.split(/[\\/]/).pop() || selectedObjectFolderPath;
    setCurrentPath([displayName]);

    // Clear selection on path switch
    clearGridSelection();
  }, [selectedObjectFolderPath, setExplorerSubPath, setCurrentPath, clearGridSelection]);

  // ── Filter & Sort ─────────────────────────────────────────────────────────
  // Safe mode filtering is handled by backend `apply_safe_mode_filter` (SSoT).
  // Frontend only applies local search filter.
  const filteredFolders = useMemo(() => {
    if (!explorerSearchQuery) return rawFolders;
    const q = explorerSearchQuery.toLowerCase();
    return rawFolders.filter((folder) => folder.name.toLowerCase().includes(q));
  }, [rawFolders, explorerSearchQuery]);

  const sortedFolders = useMemo(
    () => sortFolders(filteredFolders, sortField, sortOrder),
    [filteredFolders, sortField, sortOrder],
  );

  // ── Layout & Virtualization (extracted) ───────────────────────────────────
  const isGridView = viewMode === 'grid' && !isMobile;

  const { virtualItems, totalSize, scrollToIndex, columnCount, cardWidth } = useFolderGridLayout({
    parentRef,
    explorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    isGridView,
    itemCount: sortedFolders.length,
  });

  // ── Composed Sub-Hooks ────────────────────────────────────────────────────
  const nav = useFolderGridNav({
    currentPath,
    explorerSubPath,
    selectedObject: null, // Legacy, unused
    objects,
    setCurrentPath,
    setExplorerSubPath,
    setExplorerScrollOffset,
    clearGridSelection,
    sortField,
    sortOrder,
    setSortField,
    setSortOrder,
  });

  const actions = useFolderGridActions({ sortedFolders, objects, clearGridSelection });

  const toggleMod = useToggleMod();
  const handleToggleSelf = useCallback(
    async (enable: boolean) => {
      if (!activeGame?.id || !activeGame?.mod_path || !explorerSubPath) return;
      const targetPath = await join(activeGame.mod_path, explorerSubPath);
      toggleMod.mutate(
        { path: targetPath, enable, gameId: activeGame.id },
        {
          onSuccess: (newPath) => {
            if (!activeGame.mod_path) return;
            syncExplorerAfterRename(activeGame.mod_path, targetPath, newPath);
            queryClient.invalidateQueries({ queryKey: ['objects'] });
            queryClient.invalidateQueries({ queryKey: ['category-counts'] });
          },
        },
      );
    },
    [activeGame, explorerSubPath, toggleMod, queryClient],
  );

  // Enable the immediate parent folder (one path segment up from explorerSubPath).
  // Called from the sticky notice bar "Enable Parent" button (direct action, no dialog).
  // Enable the specific ancestor folder that is locking this view.
  const handleEnableParent = useCallback(async () => {
    if (!activeGame?.id || !activeGame?.mod_path || !ancestorDisabledPath) return;

    // Toggle the specific ancestor path that was identified by the backend as the locker
    toggleMod.mutate(
      { path: ancestorDisabledPath, enable: true, gameId: activeGame.id },
      {
        onSuccess: (newPath) => {
          if (!activeGame.mod_path) return;
          syncExplorerAfterRename(activeGame.mod_path, ancestorDisabledPath, newPath);
          queryClient.invalidateQueries({ queryKey: ['objects'] });
          queryClient.invalidateQueries({ queryKey: ['category-counts'] });
          setEnableParentDialogOpen(false);
        },
      },
    );
  }, [activeGame, ancestorDisabledPath, toggleMod, queryClient]);

  // Guarded toggle: if current directory is locked by a parent, show dialog instead
  const handleToggleEnabledGuarded = useCallback(
    (folder: ModFolder) => {
      if (ancestorDisabledBy) {
        setEnableParentDialogOpen(true);
        return;
      }
      actions.handleToggleEnabled(folder);
    },
    [ancestorDisabledBy, actions],
  );

  const bulk = useFolderGridBulk({
    gridSelection,
    sortedFolders,
    clearGridSelection,
    openMoveDialog: actions.openMoveDialog,
  });

  // ── Keyboard Navigation ──────────────────────────────────────────────────
  const [lastSelectedPath, setLastSelectedPath] = useState<string | null>(null);

  const handleToggleSelection = useCallback(
    (path: string, multi: boolean, isShift?: boolean) => {
      if (isShift && lastSelectedPath) {
        const startIdx = sortedFolders.findIndex((f) => f.path === lastSelectedPath);
        const endIdx = sortedFolders.findIndex((f) => f.path === path);

        if (startIdx !== -1 && endIdx !== -1) {
          const min = Math.min(startIdx, endIdx);
          const max = Math.max(startIdx, endIdx);

          const newSet = new Set(gridSelection);
          for (let i = min; i <= max; i++) {
            newSet.add(sortedFolders[i].path);
          }
          setGridSelection(newSet);
          // Don't update lastSelectedPath so standard shift-click behavior works from origin
          return;
        }
      }

      toggleGridSelection(path, multi);
      setLastSelectedPath(path);
    },
    [sortedFolders, gridSelection, lastSelectedPath, setGridSelection, toggleGridSelection],
  );

  const { focusedId, handleKeyDown } = useFolderNavigation({
    items: sortedFolders,
    gridColumns: isGridView ? columnCount : 1,
    getId: (item: ModFolder) => item.path,
    onNavigate: (item: ModFolder) => nav.handleNavigate(item.folder_name),
    onSelectionChange: (item: ModFolder, multi: boolean, isShift?: boolean) =>
      handleToggleSelection(item.path, multi, isShift),
    onSelectAll: () => setGridSelection(new Set(sortedFolders.map((f) => f.path))),
    onDelete: (items: ModFolder[]) => {
      if (items.length > 0) actions.handleDeleteRequest(items[0]);
    },
    onRename: (item: ModFolder) => actions.handleRenameRequest(item),
    onGoUp: () => {
      if (currentPath.length > 0) nav.handleBreadcrumbClick(currentPath.length - 2);
    },
    onFocusChange: (nextId: string | null) => {
      const idx = sortedFolders.findIndex((f) => f.path === nextId);
      if (idx !== -1) {
        const rowIndex = isGridView ? Math.floor(idx / columnCount) : idx;
        scrollToIndex(rowIndex, { align: 'auto' });
      }
    },
  });

  // ── DnD & Import (extracted) ──────────────────────────────────────────────
  const { isDragging, handleImportFiles, handleRefresh } = useFolderGridImport({
    parentRef,
    activeModPath: activeGame?.mod_path,
    explorerSubPath,
  });

  return {
    // Data & State
    rawFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    isPlaceholderData,
    selfNodeType,
    selfIsMod,
    selfIsEnabled,
    selfReasons,
    conflicts,
    // Parent-disabled lock state
    ancestorDisabledBy,
    enableParentDialogOpen,
    setEnableParentDialogOpen,
    handleEnableParent,
    handleToggleEnabledGuarded,
    isGridView,
    isMobile,
    selectedObject: null, // Legacy, unused
    currentPath,
    explorerSearchQuery,
    sortField,
    sortOrder,
    sortLabel: nav.sortLabel,
    viewMode,

    // Virtualization
    parentRef,
    virtualItems,
    totalSize,
    scrollToIndex,
    columnCount,
    cardWidth,

    // Navigation
    handleNavigate: nav.handleNavigate,
    handleBreadcrumbClick: nav.handleBreadcrumbClick,
    handleGoHome: nav.handleGoHome,
    setMobilePane,
    setViewMode,
    setExplorerSearch,
    handleSortToggle: nav.handleSortToggle,
    handleKeyDown,
    focusedId,
    handleRefresh,

    // Selection
    gridSelection,
    toggleGridSelection: handleToggleSelection,
    clearGridSelection,

    // Actions (single-item)
    ...actions,
    handleToggleSelf,

    // Duplicate Warning
    duplicateWarning: actions.duplicateWarning,
    handleDuplicateForceEnable: actions.handleDuplicateForceEnable,
    handleDuplicateEnableOnly: actions.handleDuplicateEnableOnly,
    handleDuplicateCancel: actions.handleDuplicateCancel,

    // Bulk actions
    ...bulk,

    // Objects (for MoveToObjectDialog)
    objects,

    // DnD and Import
    isDragging,
    handleImportFiles,

    // Epic 5
    isPreviewOpen,
    togglePreview,
  };
}
