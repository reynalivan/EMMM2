/**
 * useFolderGrid — Orchestrator hook for the FolderGrid component.
 *
 * Composes: useFolderGridNav, useFolderGridActions, useFolderGridBulk
 * and manages data fetching, virtualization, keyboard, DnD.
 * Refactored from 651 lines → <200 lines for 350-line compliance.
 */

import { useRef, useMemo, useState, useCallback, useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useAppStore } from '../../../stores/useAppStore';
import { useResponsive } from '../../../hooks/useResponsive';
import { useObjects } from '../../../hooks/useObjects';
import {
  useModFolders,
  sortFolders,
  folderKeys,
  useImportMods,
  ModFolder,
  useToggleMod,
} from '../../../hooks/useFolders';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useFolderNavigation } from './useFolderNavigation';
import { useFileDrop } from '../../../hooks/useFileDrop';
import { useFolderGridNav } from './useFolderGridNav';
import { useFolderGridActions } from './useFolderGridActions';
import { useFolderGridBulk } from './useFolderGridBulk';
import { useDragAutoScroll } from '../../../hooks/useDragAutoScroll';

// Grid layout constants
const CARD_MIN_W = 160;
const CARD_MAX_W = 280;
const CARD_INFO_H = 70;
const LIST_ROW_HEIGHT = 52;
const GAP = 12;

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
  const [containerWidth, setContainerWidth] = useState(800);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  // ── Data Fetching ─────────────────────────────────────────────────────────
  const {
    data: rawResponse,
    isLoading,
    isError,
    error,
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
  const prevSelectedRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedObject || selectedObject === prevSelectedRef.current) return;
    prevSelectedRef.current = selectedObject;
    const obj = objects.find((o) => o.id === selectedObject);
    if (obj) {
      setExplorerSubPath(obj.folder_path);
      setCurrentPath([obj.name]);
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

  // ── Layout & Virtualization ───────────────────────────────────────────────
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

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) setContainerWidth(entry.contentRect.width);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => (isGridView ? cardHeight + GAP : LIST_ROW_HEIGHT),
    overscan: 5,
    initialOffset: explorerScrollOffset,
  });

  useEffect(() => {
    rowVirtualizer.scrollToOffset(0);
  }, [explorerSubPath]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    let rafId: number;
    const handleScroll = () => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => setExplorerScrollOffset(el.scrollTop));
    };
    el.addEventListener('scroll', handleScroll, { passive: true });
    return () => {
      el.removeEventListener('scroll', handleScroll);
      cancelAnimationFrame(rafId);
    };
  }, [setExplorerScrollOffset]);

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
            // Extract new subPath from returned absolute path
            const cleanModPath = activeGame.mod_path.replace(/\\/g, '/');
            const cleanNewPath = newPath.replace(/\\/g, '/');
            let newSubPath = cleanNewPath.substring(cleanModPath.length);
            if (newSubPath.startsWith('/')) newSubPath = newSubPath.substring(1);
            if (newSubPath) {
              setExplorerSubPath(newSubPath);
              setCurrentPath(newSubPath.split('/'));
            }
            // Refresh ObjectList + category counts so sidebar reflects the new state
            queryClient.invalidateQueries({ queryKey: ['objects'] });
            queryClient.invalidateQueries({ queryKey: ['category-counts'] });
          },
        },
      );
    },
    [
      activeGame?.id,
      activeGame?.mod_path,
      explorerSubPath,
      toggleMod,
      setExplorerSubPath,
      setCurrentPath,
      queryClient,
    ],
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

  // ── DnD & Refresh ────────────────────────────────────────────────────────
  const importMods = useImportMods();

  const handleImportFiles = useCallback(
    async (paths: string[]) => {
      if (!activeGame?.mod_path || paths.length === 0) return;

      const { join } = await import('@tauri-apps/api/path');
      const targetDir = explorerSubPath
        ? await join(activeGame.mod_path, explorerSubPath)
        : activeGame.mod_path;

      importMods.mutate({
        paths,
        targetDir,
        strategy: 'Raw',
      });
    },
    [activeGame?.mod_path, explorerSubPath, importMods],
  );

  const { isDragging, dragPosition } = useFileDrop({ onDrop: handleImportFiles });

  useDragAutoScroll({
    containerRef: parentRef,
    dragPosition,
  });

  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: folderKeys.all });
  }, [queryClient]);

  return {
    // Data & State
    rawFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
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
