import { useShallow } from 'zustand/react/shallow';
import { useMemo, useEffect, useCallback, useRef } from 'react';
import { useAppStore } from '@/app/store';
import { useGameSchema } from './useObjectQueries';
import { useActiveGame } from '@/entities/game';
import { useResponsive } from '../../../shared/lib/hooks/useResponsive';
import { useObjectListVirtualizer } from './useObjectListVirtualizer';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useObjectBulkSelect } from './useObjectBulkSelect';
import {
  areObjectMetaFiltersEqual,
  sanitizeObjectMetaFilters,
  type FilterDef,
  type ObjectMetaFilters,
} from '@/entities/game-object';
import type { WorkspaceObjectNode } from '@/entities/workspace';
import { useWorkspaceViewModel } from '@/features/workspace-runtime';
import { DEFAULT_SOURCE_UNAVAILABLE_MESSAGE } from '@/features/workspace-runtime';
import { useWorkspaceRuntime } from '@/features/workspace-runtime';
const EMPTY_OBJECT_META_FILTERS: ObjectMetaFilters = {};
const EMPTY_WORKSPACE_OBJECTS: WorkspaceObjectNode[] = [];

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

  // Selector-scoped: a bare useAppStore() here re-runs the whole list
  // computation (filter, sort, virtualizer) on any write to any slice.
  const {
    selectedObjectFolderPath,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
    objectMetaFilters,
    setObjectMetaFilters,
    objectSortBy,
    setObjectSortBy,
    objectStatusFilter,
    setObjectStatusFilter,
    safetyFilter,
  } = useAppStore(
    useShallow((state) => ({
      selectedObjectFolderPath: state.selectedObjectFolderPath,
      selectedObjectType: state.selectedObjectType,
      setSelectedObjectType: state.setSelectedObjectType,
      sidebarSearchQuery: state.sidebarSearchQuery,
      setSidebarSearch: state.setSidebarSearch,
      objectMetaFilters: state.objectMetaFilters,
      setObjectMetaFilters: state.setObjectMetaFilters,
      objectSortBy: state.objectSortBy,
      setObjectSortBy: state.setObjectSortBy,
      objectStatusFilter: state.objectStatusFilter,
      setObjectStatusFilter: state.setObjectStatusFilter,
      safetyFilter: state.safetyFilter,
    })),
  );
  const { focusObject } = useWorkspaceRuntime();
  const activeFiltersState = objectMetaFilters ?? EMPTY_OBJECT_META_FILTERS;
  const activeSortBy = objectSortBy ?? 'name';
  const activeStatusFilter = objectStatusFilter ?? 'all';
  const filterScopeRef = useRef<string | null>(null);

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

  const filterScope = useMemo(
    () => `${selectedObjectType ?? ''}|${categoryFilters.map((filter) => filter.key).join('\0')}`,
    [categoryFilters, selectedObjectType],
  );

  useEffect(() => {
    if (!schema) {
      return;
    }

    const previousScope = filterScopeRef.current;
    filterScopeRef.current = filterScope;
    if (
      previousScope === null ||
      previousScope === filterScope ||
      areObjectMetaFiltersEqual(activeFiltersState, effectiveObjectMetaFilters)
    ) {
      return;
    }

    setObjectMetaFilters(effectiveObjectMetaFilters);
  }, [activeFiltersState, effectiveObjectMetaFilters, filterScope, schema, setObjectMetaFilters]);

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
  const allObjects = workspace?.objects ?? EMPTY_WORKSPACE_OBJECTS;
  const sourceState = workspace?.runtime?.source_state;
  const sourceUnavailableMessage =
    sourceState?.status === 'unavailable'
      ? (sourceState.message ?? DEFAULT_SOURCE_UNAVAILABLE_MESSAGE)
      : null;
  const sourceAvailable = sourceState?.status !== 'unavailable';

  // ponytail: substring filter; bring back a worker only if profiling shows lag on huge lists
  const objects = useMemo(() => {
    const query = sidebarSearchQuery.trim().toLowerCase();
    return allObjects.filter((object) => {
      if (query && !object.name.toLowerCase().includes(query)) {
        return false;
      }
      if (safetyFilter === 'safe') {
        return object.safe_mod_count > 0;
      }
      if (safetyFilter === 'unsafe') {
        return object.unsafe_mod_count > 0;
      }
      return true;
    });
  }, [allObjects, safetyFilter, sidebarSearchQuery]);

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

  // Create bulkSelect first as it's needed by handlers
  const bulkSelect = useObjectBulkSelect(flatObjectItems);

  const handlers = useObjectListHandlers({
    objects: allObjects,
    schema,
  });

  const handleFilterChange = useCallback(
    (key: string, values: string[]) => {
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
    },
    [activeFiltersState, effectiveObjectMetaFilters, setObjectMetaFilters],
  );

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
      activeGame,
      isMobile,
      isSyncing: handlers.isSyncing,
      sourceAvailable,
      sourceUnavailableMessage,
    }),
    [
      objects,
      isLoading,
      isError,
      objectsErrorInfo,
      activeGame,
      isMobile,
      handlers.isSyncing,
      sourceAvailable,
      sourceUnavailableMessage,
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

  // useObjectListHandlers returns a fresh literal each render, so memoizing
  // these two on [handlers] never hits — it only re-lists every field name.
  const modals = {
    deleteObjectDialog: handlers.deleteObjectDialog,
    setDeleteObjectDialog: handlers.setDeleteObjectDialog,
    forceDeleteObjectDialog: handlers.forceDeleteObjectDialog,
    setForceDeleteObjectDialog: handlers.setForceDeleteObjectDialog,
    editObject: handlers.editObject,
    setEditObject: handlers.setEditObject,
    bulkTagModal: handlers.bulkTagModal,
    setBulkTagModal: handlers.setBulkTagModal,
  };

  const handlerMap = {
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
    handleDropOnItem: handlers.handleDropOnItem,
    handleDropAutoOrganize: handlers.handleDropAutoOrganize,
    handleDropOnNewObjectSubmit: handlers.handleDropOnNewObjectSubmit,
    handleBulkDelete: handlers.handleBulkDelete,
    handleBulkPin: handlers.handleBulkPin,
    handleBulkEnable: handlers.handleBulkEnable,
    handleBulkDisable: handlers.handleBulkDisable,
    handleBulkAddTags: handlers.handleBulkAddTags,
    handleBulkRemoveTags: handlers.handleBulkRemoveTags,
    handleBulkClassifyAndMatch: handlers.handleBulkClassifyAndMatch,
    handleBulkFavorite: handlers.handleBulkFavorite,
    handleBulkSafe: handlers.handleBulkSafe,
  };

  return { state, filters, nav, virtualizer, modals, handlers: handlerMap, bulkSelect };
}
