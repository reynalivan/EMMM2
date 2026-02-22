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
import { useModFolders, sortFolders, folderKeys } from '../../../hooks/useFolders';
import { useFolderNavigation } from '../../../hooks/useFolderNavigation';
import { useFileDrop } from '../../../hooks/useFileDrop';
import { useFolderGridNav } from './useFolderGridNav';
import { useFolderGridActions } from './useFolderGridActions';
import { useFolderGridBulk } from './useFolderGridBulk';

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
  } = useAppStore();

  const { isMobile } = useResponsive();
  const parentRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(800);
  const queryClient = useQueryClient();

  // ── Data Fetching ─────────────────────────────────────────────────────────
  const {
    data: rawFolders = [],
    isLoading,
    isError,
    error,
  } = useModFolders(explorerSubPath, selectedObject ?? undefined);

  const { data: objects = [] } = useObjects();

  // Sync object selection → filesystem sub-path
  const prevSelectedRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedObject || selectedObject === prevSelectedRef.current) return;
    prevSelectedRef.current = selectedObject;
    const obj = objects.find((o) => o.id === selectedObject);
    if (obj) {
      setExplorerSubPath(obj.name);
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

  // eslint-disable-next-line react-hooks/incompatible-library
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

  const bulk = useFolderGridBulk({
    gridSelection,
    sortedFolders,
    clearGridSelection,
    openMoveDialog: actions.openMoveDialog,
  });

  // ── Keyboard Navigation ──────────────────────────────────────────────────
  const { focusedId, handleKeyDown } = useFolderNavigation({
    items: sortedFolders,
    gridColumns: isGridView ? columnCount : 1,
    getId: (item) => item.path,
    onNavigate: (item) => nav.handleNavigate(item.folder_name),
    onSelectionChange: (item, multi) => toggleGridSelection(item.path, multi),
    onSelectAll: () => sortedFolders.forEach((f) => toggleGridSelection(f.path, true)),
    onDelete: (items) => {
      if (items.length > 0) actions.handleDeleteRequest(items[0]);
    },
    onRename: (item) => actions.handleRenameRequest(item),
    onGoUp: () => {
      if (currentPath.length > 0) nav.handleBreadcrumbClick(currentPath.length - 2);
    },
  });

  // ── DnD & Refresh ────────────────────────────────────────────────────────
  const noopDrop = useCallback((_paths: string[]) => {}, []);
  const { isDragging } = useFileDrop({ onDrop: noopDrop });

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
    toggleGridSelection,
    clearGridSelection,

    // Actions (single-item)
    ...actions,

    // Bulk actions
    ...bulk,

    // Objects (for MoveToObjectDialog)
    objects,

    // DnD
    isDragging,
  };
}
