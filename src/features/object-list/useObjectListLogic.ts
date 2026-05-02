import { useState, useMemo, useEffect, useCallback } from 'react';
import { useAppStore } from '../../stores/useAppStore';
import { useGameSchema } from '../../hooks/useObjectQueries';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useResponsive } from '../../hooks/useResponsive';
import { useObjectListVirtualizer } from './useObjectListVirtualizer';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useObjectBulkSelect } from './useObjectBulkSelect';
import { useSearchWorker } from './hooks/useSearchWorker';
import type { FilterDef } from '../../types/object';
import type { WorkspaceObjectNode } from '../../types/workspace';
import { useWorkspaceViewModel } from '../workspace-runtime/useWorkspaceViewModel';
import { useWorkspaceRuntime } from '../workspace-runtime/state/workspaceStoreBridge';
import {
  areObjectMetaFiltersEqual,
  sanitizeObjectMetaFilters,
  type ObjectMetaFilters,
} from './objectFilterState';

/**
 * useObjectListLogic — Top-level logic for the ObjectList component.
 * Composes filtering, sorting, virtualization, and action handlers.
 *
 * Return value is namespaced into semantic groups to keep consumers clean:
 *   - state: data + loading
 *   - filters: filter/sort controls
 *   - nav: selection/search
 *   - virtualizer: scroll/list state
 *   - modals: dialog open/close state
 *   - handlers: all event handlers
 *   - bulkSelect: bulk selection instance
 */
export function useObjectListLogic() {
  const { isMobile } = useResponsive();
  const { activeGame } = useActiveGame();

  const {
    selectedObjectFolderPath,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
    safeMode,
    objectMetaFilters,
    setObjectMetaFilters,
    objectSortBy,
    setObjectSortBy,
    objectStatusFilter,
    setObjectStatusFilter,
  } = useAppStore();
  const { focusObject } = useWorkspaceRuntime();
  const activeFiltersState = objectMetaFilters ?? {};
  const activeSortBy = objectSortBy ?? 'name';
  const activeStatusFilter = objectStatusFilter ?? 'all';

  const { filteredIds, search: workerSearch } = useSearchWorker();
  const { data: schema } = useGameSchema();

  const categoryFilters: FilterDef[] = useMemo(() => {
    if (!schema) return [];
    if (selectedObjectType) {
      const cat = schema.categories.find((c) => c.name === selectedObjectType);
      return cat?.filters ?? [];
    }
    const seen = new Map<string, FilterDef>();
    for (const cat of schema.categories) {
      for (const f of cat.filters ?? []) {
        if (!seen.has(f.key)) seen.set(f.key, f);
      }
    }
    return [...seen.values()];
  }, [schema, selectedObjectType]);

  const effectiveObjectMetaFilters = useMemo<ObjectMetaFilters>(() => {
    if (!schema) {
      return activeFiltersState;
    }

    return sanitizeObjectMetaFilters(activeFiltersState, categoryFilters);
  }, [activeFiltersState, categoryFilters, schema]);

  const {
    data: workspace,
    isLoading: objectsLoading,
    isError: objectsError,
    error: objectsErrorInfo,
  } = useWorkspaceViewModel({
    filterOverrides: {
      objectMetaFilters: effectiveObjectMetaFilters,
    },
  });
  const allObjects = (workspace?.objects ?? []) as WorkspaceObjectNode[];

  // Fix 2: Memoize search items outside the effect to avoid redundant array creation.
  const searchItems = useMemo(
    () => allObjects.map((o) => ({ id: o.id, name: o.name })),
    [allObjects],
  );

  useEffect(() => {
    const t = setTimeout(() => workerSearch(searchItems, sidebarSearchQuery), 150);
    return () => clearTimeout(t);
  }, [searchItems, sidebarSearchQuery, workerSearch]);

  const objects = useMemo(() => {
    if (!filteredIds) return allObjects;
    return allObjects.filter((o) => filteredIds.has(o.id));
  }, [allObjects, filteredIds]);

  const isLoading = objectsLoading;
  const isError = objectsError;

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

  // Create bulkSelect first as it's needed by handlers
  const bulkSelect = useObjectBulkSelect(flatObjectItems);

  const handlers = useObjectListHandlers({
    objects: allObjects,
    schema,
    mismatchConfirm,
    setMismatchConfirm,
  });

  const handleFilterChange = useCallback((key: string, values: string[]) => {
    const nextFilters: ObjectMetaFilters = { ...effectiveObjectMetaFilters };
    if (values.length === 0) {
      delete nextFilters[key];
    } else {
      nextFilters[key] = values;
    }

    if (areObjectMetaFiltersEqual(activeFiltersState, nextFilters)) {
      return;
    }

    setObjectMetaFilters(nextFilters);
  }, [activeFiltersState, effectiveObjectMetaFilters, setObjectMetaFilters]);

  const handleClearFilters = useCallback(() => {
    if (Object.keys(activeFiltersState).length === 0) {
      return;
    }

    setObjectMetaFilters({});
  }, [activeFiltersState, setObjectMetaFilters]);

  const handleSelectObject = useCallback(
    (folderPath: string) => {
      focusObject(folderPath);
    },
    [focusObject],
  );

  // ── Namespaced Return Value ─────────────────────────────────────────
  // Fix 4: Group into semantic namespaces instead of a flat 40+ property object.
  // Consumers should destructure the namespace they need (e.g. `state`, `handlers`).

  const state = useMemo(
    () => ({
      objects,
      isLoading,
      isError,
      objectsErrorInfo: isError ? objectsErrorInfo : null,
      safeMode,
      activeGame,
      isMobile,
      isSyncing: handlers.isSyncing,
    }),
    [
      objects,
      isLoading,
      isError,
      objectsErrorInfo,
      safeMode,
      activeGame,
      isMobile,
      handlers.isSyncing,
    ],
  );

  const filters = useMemo(
    () => ({
      activeFilters: effectiveObjectMetaFilters,
      categoryFilters,
      schema,
      sortBy: activeSortBy,
      setSortBy: setObjectSortBy,
      statusFilter: activeStatusFilter,
      setStatusFilter: setObjectStatusFilter,
      handleFilterChange,
      handleClearFilters,
    }),
    [
      effectiveObjectMetaFilters,
      categoryFilters,
      schema,
      activeSortBy,
      setObjectSortBy,
      activeStatusFilter,
      setObjectStatusFilter,
      handleFilterChange,
      handleClearFilters,
    ],
  );

  const nav = useMemo(
    () => ({
      selectedObjectFolderPath,
      selectObject: handleSelectObject,
      selectedObjectType,
      setSelectedObjectType,
      sidebarSearchQuery,
      setSidebarSearch,
    }),
    [
      selectedObjectFolderPath,
      handleSelectObject,
      selectedObjectType,
      setSelectedObjectType,
      sidebarSearchQuery,
      setSidebarSearch,
    ],
  );

  // parentRef is a stable useRef — no need to memo the whole virtualizer object.
  // The values inside are already memoized by useObjectListVirtualizer itself.
  const virtualizer = {
    parentRef,
    rowVirtualizer,
    flatObjectItems,
    totalItems,
    stickyPosition,
    selectedIndex,
    scrollToSelected,
  };

  const modals = useMemo(
    () => ({
      deleteObjectDialog: handlers.deleteObjectDialog,
      setDeleteObjectDialog: handlers.setDeleteObjectDialog,
      forceDeleteObjectDialog: handlers.forceDeleteObjectDialog,
      setForceDeleteObjectDialog: handlers.setForceDeleteObjectDialog,
      editObject: handlers.editObject,
      setEditObject: handlers.setEditObject,
      syncConfirm: handlers.syncConfirm,
      setSyncConfirm: handlers.setSyncConfirm,
      scanReview: handlers.scanReview,
      archiveModal: handlers.archiveModal,
      bulkTagModal: handlers.bulkTagModal,
      setBulkTagModal: handlers.setBulkTagModal,
      mismatchConfirm,
      setMismatchConfirm,
    }),
    [handlers, mismatchConfirm],
  );

  const handlerMap = useMemo(
    () => ({
      handleDeleteObject: handlers.handleDeleteObject,
      confirmDeleteObject: handlers.confirmDeleteObject,
      confirmForceDeleteObject: handlers.confirmForceDeleteObject,
      handleEdit: handlers.handleEdit,
      handlePin: handlers.handlePin,
      handleMoveCategory: handlers.handleMoveCategory,
      handleRevealInExplorer: handlers.handleRevealInExplorer,
      handleEnableObject: handlers.handleEnableObject,
      handleDisableObject: handlers.handleDisableObject,
      isSwitchPending: handlers.isSwitchPending,
      isObjectSwitchPending: handlers.isObjectSwitchPending,
      categoryNames: handlers.categoryNames,
      handleSync: handlers.handleSync,
      handleBackgroundSync: handlers.handleBackgroundSync,
      handleSyncWithDb: handlers.handleSyncWithDb,
      handleApplySyncMatch: handlers.handleApplySyncMatch,
      handleCommitScan: handlers.handleCommitScan,
      handleCloseScanReview: handlers.handleCloseScanReview,
      handleDropOnItem: handlers.handleDropOnItem,
      handleDropAutoOrganize: handlers.handleDropAutoOrganize,
      handleDropOnNewObjectSubmit: handlers.handleDropOnNewObjectSubmit,
      handleArchivesInteractively: handlers.handleArchivesInteractively,
      handleArchiveExtractSubmit: handlers.handleArchiveExtractSubmit,
      handleArchiveExtractSkip: handlers.handleArchiveExtractSkip,
      handleStopExtraction: handlers.handleStopExtraction,
      handleBulkDelete: handlers.handleBulkDelete,
      handleBulkPin: handlers.handleBulkPin,
      handleBulkEnable: handlers.handleBulkEnable,
      handleBulkDisable: handlers.handleBulkDisable,
      handleBulkAddTags: handlers.handleBulkAddTags,
      handleBulkRemoveTags: handlers.handleBulkRemoveTags,
      handleBulkAutoOrganize: handlers.handleBulkAutoOrganize,
      handleBulkFavorite: handlers.handleBulkFavorite,
      handleBulkSafe: handlers.handleBulkSafe,
    }),
    [handlers],
  );

  return { state, filters, nav, virtualizer, modals, handlers: handlerMap, bulkSelect };
}
