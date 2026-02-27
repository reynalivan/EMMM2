import { useState, useMemo, useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../stores/useAppStore';
import { useObjects, useGameSchema } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useResponsive } from '../../hooks/useResponsive';
import { useObjectListVirtualizer } from './useObjectListVirtualizer';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useSearchWorker } from '../../hooks/useSearchWorker';
import { scanService } from '../../services/scanService';
import { toast } from '../../stores/useToastStore';
import type { FilterDef } from '../../types/object';

export function useObjectListLogic() {
  const { isMobile } = useResponsive();
  const { activeGame } = useActiveGame();

  const {
    selectedObject,
    setSelectedObject,
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

  // Auto-sync: when a game is selected but DB has 0 objects, quick-import all as "Other".
  // Then offer an "Auto Organize" toast so the user can opt-in to full Deep Matcher matching.
  const queryClient = useQueryClient();
  const autoSyncTriggered = useRef<string | null>(null);

  // US-3.6: Trigger worker search when query or data changes
  useEffect(() => {
    const items = allObjects.map((o) => ({ id: o.id, name: o.name }));
    workerSearch(items, sidebarSearchQuery);
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
    selectedObject,
    isMobile,
  });

  // --- Delegated: Handlers (use allObjects for reliable ID lookups) ---
  const handlers = useObjectListHandlers({
    objects: allObjects,
    schema,
  });

  // Auto-sync: when a game is selected but DB has 0 mods, quick-import all as "Other".
  // Then offer an "Auto Organize" toast so the user can opt-in to full Deep Matcher matching.
  // We check the DB directly (not React query) to avoid false triggers during loading transitions.
  useEffect(() => {
    if (
      activeGame &&
      !objectsLoading &&
      !objectsError &&
      autoSyncTriggered.current !== activeGame.id
    ) {
      autoSyncTriggered.current = activeGame.id;
      (async () => {
        try {
          // Direct DB check: only import if there are truly 0 mods for this game
          const { getObjects } = await import('../../services/objectService');
          const allGameObjects = await getObjects({
            game_id: activeGame.id,
            safe_mode: false,
          });
          const existingModCount = allGameObjects.reduce((acc, obj) => acc + obj.mod_count, 0);
          if (existingModCount > 0) {
            // Mods already exist in DB — skip quickImport, just refresh queries
            queryClient.invalidateQueries({ queryKey: ['objects'] });
            queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
            return;
          }

          const result = await scanService.quickImport(
            activeGame.id,
            activeGame.name,
            activeGame.game_type,
            activeGame.mod_path,
          );
          queryClient.invalidateQueries({ queryKey: ['objects'] });
          queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
          queryClient.invalidateQueries({ queryKey: ['category-counts'] });
          if (result.new_mods > 0) {
            toast.withAction(
              'info',
              `Imported ${result.new_mods} mods as "Other". Want to auto-organize them?`,
              {
                label: 'Auto Organize',
                onClick: () => handlers.handleSync(),
              },
              8000,
            );
          }
        } catch (e) {
          console.error('Quick import failed:', e);
        }
      })();
    }
  }, [activeGame, objectsLoading, objectsError, queryClient, handlers]);

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

  return {
    // Refs
    parentRef,

    // State
    isMobile,
    activeGame,
    selectedObject,
    setSelectedObject,
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
    handleArchiveExtractSubmit: handlers.handleArchiveExtractSubmit,
    handleArchiveExtractSkip: handlers.handleArchiveExtractSkip,
  };
}
