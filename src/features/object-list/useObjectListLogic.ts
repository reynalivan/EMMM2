import { useState, useMemo, useEffect } from 'react';

import { useAppStore } from '../../stores/useAppStore';
import { useObjects, useGameSchema } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useResponsive } from '../../hooks/useResponsive';
import { useObjectListVirtualizer } from './useObjectListVirtualizer';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useObjectBulkSelect } from './useObjectBulkSelect';
import { useSearchWorker } from './hooks/useSearchWorker';

import type { FilterDef } from '../../types/object';

export function useObjectListLogic() {
  const { isMobile } = useResponsive();
  const { activeGame } = useActiveGame();

  const {
    selectedObjectFolderPath,
    setSelectedObjectFolderPath,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
    safeMode,
    setSafeMode,
  } = useAppStore();

  // Active filters state (schema-driven)
  const [activeFilters, setActiveFilters] = useState<Record<string, string[]>>({});
  const [sortBy, setSortBy] = useState<'name' | 'date' | 'rarity'>('name');
  const [statusFilter, setStatusFilter] = useState<'all' | 'enabled' | 'disabled'>('all');

  // US-3.6: Web Worker search
  const { filteredIds, search: workerSearch } = useSearchWorker();

  // Data hooks — dual source (SQL search delegated to worker)
  const {
    data: allObjects = [],
    isLoading: objectsLoading,
    isError: objectsError,
    error: objectsErrorInfo,
  } = useObjects({
    metaFilters: activeFilters,
    sortBy,
    statusFilter,
    localSearch: true,
  });

  // US-3.6: Trigger worker search when query or data changes (debounced)
  useEffect(() => {
    const timeout = setTimeout(() => {
      const items = allObjects.map((o) => ({ id: o.id, name: o.name }));
      workerSearch(items, sidebarSearchQuery);
    }, 150);
    return () => clearTimeout(timeout);
  }, [allObjects, sidebarSearchQuery, workerSearch]);

  // Apply worker search filter
  const objects = useMemo(() => {
    if (!filteredIds) return allObjects;
    return allObjects.filter((o) => filteredIds.has(o.id));
  }, [allObjects, filteredIds]);

  const { data: schema } = useGameSchema();

  // Per-category filter adaptation: show only relevant filters for the selected category.
  // If no category selected, merge unique filters from all categories.
  const categoryFilters: FilterDef[] = useMemo(() => {
    if (!schema) return [];
    if (selectedObjectType) {
      const cat = schema.categories.find((c) => c.name === selectedObjectType);
      return cat?.filters ?? [];
    }
    // No specific category — union all category-level filters (deduplicate by key)
    const seen = new Map<string, FilterDef>();
    for (const cat of schema.categories) {
      for (const f of cat.filters ?? []) {
        if (!seen.has(f.key)) seen.set(f.key, f);
      }
    }
    return [...seen.values()];
  }, [schema, selectedObjectType]);

  const isLoading = objectsLoading;
  const isError = objectsError;

  // --- Delegated: Virtualizer & Object Mode shaping ---
  const {
    parentRef,
    rowVirtualizer,
    flatObjectItems,
    totalItems,
    stickyPosition,
    selectedIndex,
    scrollToSelected,
  } = useObjectListVirtualizer({
    objects,
    schema,
    selectedObjectFolderPath,
    isMobile,
  });

  const [mismatchConfirm, setMismatchConfirm] = useState<string[] | null>(null);

  // --- Delegated: Handlers (use allObjects for reliable ID lookups) ---
  const handlers = useObjectListHandlers({
    objects: allObjects,
    schema,
    mismatchConfirm,
    setMismatchConfirm,
  });

  const handleFilterChange = (key: string, values: string[]) => {
    setActiveFilters((prev) => ({ ...prev, [key]: values }));
  };

  // Clear stale filter keys when category changes (e.g., "element" doesn't apply to Weapon)
  const [prevCategoryFilters, setPrevCategoryFilters] = useState(categoryFilters);
  if (categoryFilters !== prevCategoryFilters) {
    setPrevCategoryFilters(categoryFilters);
    const validKeys = new Set(categoryFilters.map((f) => f.key));
    const next: Record<string, string[]> = {};
    for (const [k, v] of Object.entries(activeFilters)) {
      if (validKeys.has(k)) next[k] = v;
    }
    if (Object.keys(next).length !== Object.keys(activeFilters).length) {
      setActiveFilters(next);
    }
  }

  const handleClearFilters = () => {
    setActiveFilters({});
  };

  // Bulk selection
  const bulkSelect = useObjectBulkSelect(flatObjectItems);

  return {
    // Refs
    parentRef,

    // State
    isMobile,
    activeGame,
    selectedObjectFolderPath,
    setSelectedObjectFolderPath,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
    deleteDialog: handlers.deleteDialog,
    setDeleteDialog: handlers.setDeleteDialog,
    activeFilters,
    sortBy,
    setSortBy,
    statusFilter,
    setStatusFilter,

    // Data
    objects,
    schema,
    categoryFilters,
    isLoading,
    isError,
    objectsErrorInfo: objectsError ? objectsErrorInfo : null,

    // Virtualizer API
    rowVirtualizer,
    flatObjectItems,
    totalItems,

    // Sticky
    stickyPosition,
    selectedIndex,
    scrollToSelected,

    // Handlers
    handleToggle: handlers.handleToggle,
    handleOpen: handlers.handleOpen,
    handleDelete: handlers.handleDelete,
    confirmDelete: handlers.confirmDelete,
    handleDeleteObject: handlers.handleDeleteObject,
    deleteObjectDialog: handlers.deleteObjectDialog,
    setDeleteObjectDialog: handlers.setDeleteObjectDialog,
    confirmDeleteObject: handlers.confirmDeleteObject,
    handleFilterChange,
    handleClearFilters,

    // Edit API
    editObject: handlers.editObject,
    setEditObject: handlers.setEditObject,
    handleEdit: handlers.handleEdit,

    safeMode,
    setSafeMode,

    // Sync
    handleSync: handlers.handleSync,
    isSyncing: handlers.isSyncing,
    handleSyncWithDb: handlers.handleSyncWithDb,
    handleApplySyncMatch: handlers.handleApplySyncMatch,
    syncConfirm: handlers.syncConfirm,
    setSyncConfirm: handlers.setSyncConfirm,
    scanReview: handlers.scanReview,
    handleCommitScan: handlers.handleCommitScan,
    handleCloseScanReview: handlers.handleCloseScanReview,

    // Pin & Category
    handlePin: handlers.handlePin,
    handleFavorite: handlers.handleFavorite,
    handleMoveCategory: handlers.handleMoveCategory,
    handleRevealInExplorer: handlers.handleRevealInExplorer,
    handleEnableObject: handlers.handleEnableObject,
    handleDisableObject: handlers.handleDisableObject,
    categoryNames: handlers.categoryNames,
    handleDrop: handlers.handleDropOnItem, // legacy compat
    handleDropOnItem: handlers.handleDropOnItem,
    handleDropAutoOrganize: handlers.handleDropAutoOrganize,
    handleDropNewObject: handlers.handleDropNewObject,
    handleDropOnNewObjectSubmit: handlers.handleDropOnNewObjectSubmit,

    // Archive Modal
    archiveModal: handlers.archiveModal,
    handleArchivesInteractively: handlers.handleArchivesInteractively,
    handleArchiveExtractSubmit: handlers.handleArchiveExtractSubmit,
    handleArchiveExtractSkip: handlers.handleArchiveExtractSkip,
    handleStopExtraction: handlers.handleStopExtraction,

    // Bulk Select
    bulkSelect,

    // Mismatch Confirmation
    mismatchConfirm,
    setMismatchConfirm,
    bulkTagModal: handlers.bulkTagModal,
    setBulkTagModal: handlers.setBulkTagModal,
    handleBulkDelete: handlers.handleBulkDelete,
    handleBulkPin: handlers.handleBulkPin,
    handleBulkEnable: handlers.handleBulkEnable,
    handleBulkDisable: handlers.handleBulkDisable,
    handleBulkAddTags: handlers.handleBulkAddTags,
    handleBulkRemoveTags: handlers.handleBulkRemoveTags,
    handleBulkAutoOrganize: handlers.handleBulkAutoOrganize,
  };
}
