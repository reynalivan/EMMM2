/**
 * Epic 3: ObjectList — main sidebar component for browsing game objects.
 * Replaces mock data with real TanStack Query hooks.
 * Groups objects by category with collapsible sections.
 * Uses @tanstack/react-virtual for 500+ item performance.
 * Covers: TC-3.1-01, TC-3.5 (Virtualization), EC-3.05 (Safe Mode)
 */

import { useRef, useMemo, useState, useCallback } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Search, Loader2, AlertCircle, FolderOpen } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';
import { useObjects, useGameSchema, useCategoryCounts } from '../../hooks/useObjects';
import { useResponsive } from '../../hooks/useResponsive';
import ObjectRow from './ObjectRow';
import CategorySection from './CategorySection';
import FilterPanel from './FilterPanel';
import type { ObjectSummary } from '../../types/object';

export default function ObjectList() {
  const parentRef = useRef<HTMLDivElement>(null);
  const { isMobile } = useResponsive();
  const {
    selectedObject,
    setSelectedObject,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
  } = useAppStore();

  // Active filters state (schema-driven)
  const [activeFilters, setActiveFilters] = useState<Record<string, string[]>>({});

  // Data hooks
  const { data: objects = [], isLoading, isError, error } = useObjects();
  const { data: schema } = useGameSchema();
  const { data: categoryCounts = [] } = useCategoryCounts();

  // Build category count map
  const countMap = useMemo(() => {
    const map: Record<string, number> = {};
    for (const cc of categoryCounts) {
      map[cc.object_type] = cc.count;
    }
    return map;
  }, [categoryCounts]);

  // Group objects by category
  const groupedObjects = useMemo(() => {
    const groups: Record<string, ObjectSummary[]> = {};
    for (const obj of objects) {
      const key = obj.object_type;
      if (!groups[key]) groups[key] = [];
      groups[key].push(obj);
    }
    return groups;
  }, [objects]);

  // Flatten for virtualizer: headers + rows
  const flatItems = useMemo(() => {
    const categories = schema?.categories ?? [
      { name: 'Character', icon: 'User', color: 'primary' },
      { name: 'Weapon', icon: 'Sword', color: 'secondary' },
      { name: 'UI', icon: 'Layout', color: 'accent' },
      { name: 'Other', icon: 'Package', color: 'neutral' },
    ];

    const items: Array<
      | { type: 'header'; category: (typeof categories)[0]; count: number }
      | { type: 'row'; obj: ObjectSummary }
    > = [];

    for (const cat of categories) {
      const catObjects = groupedObjects[cat.name] ?? [];
      const count = countMap[cat.name] ?? 0;

      // Always show header
      items.push({ type: 'header', category: cat, count });

      // Show rows only if not collapsed
      const { collapsedCategories } = useAppStore.getState();
      if (!collapsedCategories.has(cat.name)) {
        for (const obj of catObjects) {
          items.push({ type: 'row', obj });
        }
      }
    }

    return items;
  }, [groupedObjects, countMap, schema]);

  // Virtualizer
  const rowVirtualizer = useVirtualizer({
    count: flatItems.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      const item = flatItems[index];
      if (item?.type === 'header') return 36;
      return isMobile ? 52 : 40;
    },
    overscan: 10,
  });

  // Handlers
  const handleFilterChange = useCallback((key: string, values: string[]) => {
    setActiveFilters((prev) => ({ ...prev, [key]: values }));
  }, []);

  const handleClearFilters = useCallback(() => {
    setActiveFilters({});
  }, []);

  return (
    <div className="flex flex-col h-full bg-base-100/50">
      {/* Search bar */}
      <div className="p-2 border-b border-base-300/30">
        <div className="relative">
          <Search
            size={14}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/30"
          />
          <input
            type="text"
            placeholder="Search objects..."
            className="input input-sm w-full pl-8 bg-base-200/40 border-base-300/20 focus:border-primary/40 text-sm"
            value={sidebarSearchQuery}
            onChange={(e) => setSidebarSearch(e.target.value)}
          />
        </div>
      </div>

      {/* Filter panel */}
      {schema && schema.filters.length > 0 && (
        <FilterPanel
          filters={schema.filters}
          activeFilters={activeFilters}
          onFilterChange={handleFilterChange}
          onClearAll={handleClearFilters}
        />
      )}

      {/* Loading state */}
      {isLoading && (
        <div className="flex-1 flex items-center justify-center" data-testid="loading-spinner">
          <Loader2 size={24} className="animate-spin text-primary/50" />
        </div>
      )}

      {/* Error state */}
      {isError && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 p-4">
          <AlertCircle size={24} className="text-error/50" />
          <p className="text-xs text-base-content/50 text-center">
            {error?.message ?? 'Failed to load objects'}
          </p>
        </div>
      )}

      {/* Empty state */}
      {!isLoading && !isError && objects.length === 0 && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 p-4">
          <FolderOpen size={32} className="text-base-content/20" />
          <p className="text-xs text-base-content/40 text-center">
            {sidebarSearchQuery
              ? 'No objects match your search'
              : 'No objects yet. Scan a mod folder to get started.'}
          </p>
        </div>
      )}

      {/* Virtualized list */}
      {!isLoading && !isError && objects.length > 0 && (
        <div ref={parentRef} className="flex-1 overflow-auto px-1.5 py-1">
          <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
            {rowVirtualizer.getVirtualItems().map((virtualItem) => {
              const item = flatItems[virtualItem.index];
              if (!item) return null;

              if (item.type === 'header') {
                return (
                  <div
                    key={`cat-${item.category.name}`}
                    className="absolute top-0 left-0 w-full"
                    style={{
                      height: `${virtualItem.size}px`,
                      transform: `translateY(${virtualItem.start}px)`,
                    }}
                  >
                    <CategorySection
                      category={item.category}
                      count={item.count}
                      isSelected={selectedObjectType === item.category.name}
                      onSelect={() => {
                        setSelectedObjectType(
                          selectedObjectType === item.category.name ? null : item.category.name,
                        );
                      }}
                    >
                      {/* Children rendered inline below via rows */}
                      <></>
                    </CategorySection>
                  </div>
                );
              }

              return (
                <div
                  key={item.obj.id}
                  className="absolute top-0 left-0 w-full pl-2"
                  style={{
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <ObjectRow
                    obj={item.obj}
                    isSelected={selectedObject === item.obj.id}
                    isMobile={isMobile}
                    onClick={() => setSelectedObject(item.obj.id)}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Status bar */}
      <div className="px-3 py-1.5 border-t border-base-300/20 flex items-center justify-between">
        <span className="text-[10px] text-base-content/30">
          {objects.length} object{objects.length !== 1 ? 's' : ''}
        </span>
        {selectedObjectType && (
          <button
            className="text-[10px] text-primary/60 hover:text-primary transition-colors"
            onClick={() => setSelectedObjectType(null)}
          >
            Show All
          </button>
        )}
      </div>
    </div>
  );
}
