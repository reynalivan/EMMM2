/**
 * useFolderGrid — Orchestrator hook for the FolderGrid component.
 *
 * Composes: useFolderGridNav, useFolderGridActions, useFolderGridBulk,
 *           useFolderGridLayout, useFolderGridImport
 * and manages data fetching, keyboard, and selection.
 */

import { useRef, useMemo, useState, useCallback, useEffect } from 'react';
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
  'use no memo';
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
  } = useModFolders(explorerSubPath, selectedObject ?? undefined);

  const rawFolders = useMemo(
    () => rawResponse?.children || ([] as ModFolder[]),
    [rawResponse?.children],
  );
  const selfNodeType = rawResponse?.self_node_type || null;
  const selfIsMod = rawResponse?.self_is_mod ?? false;
  const selfIsEnabled = rawResponse?.self_is_enabled ?? false;
  const selfReasons = rawResponse?.self_classification_reasons || [];
  const conflicts = rawResponse?.conflicts || [];

  const { data: objects = [] } = useObjects();

  // Sync object selection → filesystem sub-path
  // Uses composite key so it also re-syncs when folder_path changes (e.g., toggle rename)
  const prevSyncKeyRef = useRef<string | null>(null);
  useEffect(() => {
    const obj = objects.find((o) => o.id === selectedObject);
    if (!selectedObject || !obj) return;

    const syncKey = `${selectedObject}::${obj.folder_path}`;
    if (syncKey === prevSyncKeyRef.current) return;

    // Detect if this is a new object vs. same object with changed folder_path
    const prevObjId = prevSyncKeyRef.current?.split('::')[0];
    prevSyncKeyRef.current = syncKey;

    setExplorerSubPath(obj.folder_path);
    setCurrentPath([obj.name]);
    // Only clear selection on actual object switch, not folder_path rename
    if (prevObjId !== selectedObject) {
      clearGridSelection();
    }
  }, [selectedObject, objects, setExplorerSubPath, setCurrentPath, clearGridSelection]);

  // ── Filter & Sort ─────────────────────────────────────────────────────────
  const filteredFolders = useMemo(() => {
    const safeModeFiltered = safeMode ? rawFolders.filter((folder) => folder.is_safe) : rawFolders;
    if (!explorerSearchQuery) return safeModeFiltered;
    const q = explorerSearchQuery.toLowerCase();
    return safeModeFiltered.filter((folder) => folder.name.toLowerCase().includes(q));
  }, [rawFolders, safeMode, explorerSearchQuery]);

  const sortedFolders = useMemo(
    () => sortFolders(filteredFolders, sortField, sortOrder),
    [filteredFolders, sortField, sortOrder],
  );

  // ── Layout & Virtualization (extracted) ───────────────────────────────────
  const isGridView = viewMode === 'grid' && !isMobile;

  const { rowVirtualizer, columnCount, cardWidth } = useFolderGridLayout({
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
    selectedObject,
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

  const actions = useFolderGridActions({ sortedFolders, clearGridSelection });

  const toggleMod = useToggleMod();
  const handleToggleSelf = useCallback(
    async (enable: boolean) => {
      if (!activeGame?.id || !activeGame?.mod_path || !explorerSubPath) return;
      const { join } = await import('@tauri-apps/api/path');
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
    [activeGame?.id, activeGame?.mod_path, explorerSubPath, toggleMod, queryClient],
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
    onSelectAll: () => sortedFolders.forEach((f) => handleToggleSelection(f.path, true)),
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
        rowVirtualizer.scrollToIndex(rowIndex, { align: 'auto' });
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
    isGridView,
    isMobile,
    selectedObject,
    currentPath,
    explorerSearchQuery,
    sortField,
    sortOrder,
    sortLabel: nav.sortLabel,
    viewMode,

    // Virtualization
    parentRef,
    rowVirtualizer,
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
