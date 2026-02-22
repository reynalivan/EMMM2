/**
 * ObjectListToolbar — compact search bar + filter toggle + sync + create.
 * Category/Sort/Status filtering is fully delegated to FilterPanel.
 */

import { Search, RefreshCw, RotateCcw, Plus, SlidersHorizontal, X, Sparkles } from 'lucide-react';
import { useState, useMemo } from 'react';
import type { GameSchema, FilterDef, CategoryDef } from '../../types/object';
import FilterPanel from './FilterPanel';

interface ToolbarProps {
  sidebarSearchQuery: string;
  onSearchChange: (query: string) => void;
  schema: GameSchema | undefined;
  selectedObjectType: string | null;
  onSelectObjectType: (type: string | null) => void;
  sortBy: 'name' | 'date' | 'rarity';
  onSortChange: (val: 'name' | 'date' | 'rarity') => void;
  isSyncing: boolean;
  onSync: () => void;
  onRefresh: () => void;
  onCreateNew: () => void;
  /** Per-category filters for FilterPanel */
  categoryFilters: FilterDef[];
  activeFilters: Record<string, string[]>;
  onFilterChange: (key: string, values: string[]) => void;
  onClearFilters: () => void;
  statusFilter: 'all' | 'enabled' | 'disabled';
  onStatusFilterChange: (val: 'all' | 'enabled' | 'disabled') => void;
  showFilterPanel: boolean;
  /** True when any file is dragged over the app */
  isDragging?: boolean;
  /** True when files are being specifically dragged over this toolbar zone */
  isActiveZone?: boolean;
}

export default function ObjectListToolbar({
  sidebarSearchQuery,
  onSearchChange,
  schema,
  selectedObjectType,
  onSelectObjectType,
  sortBy,
  onSortChange,
  isSyncing,
  onSync,
  onRefresh,
  onCreateNew,
  categoryFilters,
  activeFilters,
  onFilterChange,
  onClearFilters,
  statusFilter,
  onStatusFilterChange,
  showFilterPanel,
  isDragging,
  isActiveZone,
}: ToolbarProps) {
  const [filterOpen, setFilterOpen] = useState(true);

  /** Count of active filters for badge */
  const activeCount = useMemo(() => {
    const metaCount = Object.values(activeFilters).reduce((sum, arr) => sum + arr.length, 0);
    const statusActive = statusFilter !== 'all' ? 1 : 0;
    const categoryActive = selectedObjectType ? 1 : 0;
    return metaCount + statusActive + categoryActive;
  }, [activeFilters, statusFilter, selectedObjectType]);

  const categories: CategoryDef[] = schema?.categories ?? [];

  return (
    <>
      {/* Compact toolbar: Search + Filter + Sync + Create */}
      <div className="p-2 border-b border-base-300/30 flex items-center gap-1.5 relative">
        {/* Auto Organize drop overlay — slides in from top, solid on hover */}
        {isDragging && (
          <div
            className={`absolute inset-0 z-20 flex items-center justify-center rounded-lg transition-all duration-300 animate-[slideDown_200ms_ease-out] ${
              isActiveZone
                ? 'bg-base-300 border-2 border-primary shadow-lg'
                : 'bg-base-200 border-2 border-dashed border-base-300/50'
            }`}
            style={{ animation: 'slideDown 200ms ease-out' }}
          >
            <div
              className={`flex items-center gap-2 ${isActiveZone ? 'text-primary font-bold' : 'text-base-content/50'}`}
            >
              <Sparkles size={20} className={isActiveZone ? 'animate-pulse' : ''} />
              <span className="text-sm font-semibold">Auto Organize</span>
            </div>
          </div>
        )}
        <div className="relative flex-1 min-w-0">
          <Search
            size={14}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/30"
          />
          <input
            type="text"
            placeholder="Search..."
            className="input input-sm w-full pl-8 bg-base-200/40 border-base-300/20 focus:border-primary/40 text-sm"
            value={sidebarSearchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
          />
        </div>

        {/* Filter toggle */}
        {showFilterPanel && (
          <button
            className={`btn btn-sm btn-square relative transition-all duration-200 ${
              filterOpen
                ? 'btn-primary btn-outline'
                : activeCount > 0
                  ? 'btn-ghost text-primary'
                  : 'btn-ghost text-base-content/50 hover:text-primary'
            }`}
            onClick={() => setFilterOpen((prev) => !prev)}
            title={filterOpen ? 'Hide Filters' : 'Show Filters'}
          >
            <SlidersHorizontal size={15} />
            {activeCount > 0 && !filterOpen && (
              <span className="absolute -top-1 -right-1 badge badge-xs badge-primary font-bold">
                {activeCount}
              </span>
            )}
          </button>
        )}

        <button
          className="btn btn-sm btn-square btn-ghost text-base-content/50 hover:text-primary"
          onClick={onRefresh}
          title="Refresh list"
        >
          <RotateCcw size={15} />
        </button>
        <button
          className={`btn btn-sm btn-square btn-ghost ${isSyncing ? 'animate-spin' : ''} text-base-content/50 hover:text-primary`}
          onClick={onSync}
          title="Auto Reorganize (Full scan & sync)"
          disabled={isSyncing}
        >
          <RefreshCw size={16} />
        </button>
        <button
          className="btn btn-sm btn-square btn-ghost text-base-content/50 hover:text-primary"
          onClick={onCreateNew}
          title="Create New Object"
        >
          <Plus size={16} />
        </button>
      </div>

      {/* Filter panel — collapsible, with category + sort + status + metadata */}
      {showFilterPanel && filterOpen && (
        <FilterPanel
          filters={categoryFilters}
          activeFilters={activeFilters}
          onFilterChange={onFilterChange}
          onClearAll={onClearFilters}
          statusFilter={statusFilter}
          onStatusFilterChange={onStatusFilterChange}
          categories={categories}
          selectedCategory={selectedObjectType}
          onSelectCategory={onSelectObjectType}
          sortBy={sortBy}
          onSortChange={onSortChange}
        />
      )}

      {/* Active filter chips summary — visible when panel is closed but filters active */}
      {showFilterPanel && !filterOpen && activeCount > 0 && (
        <div className="px-2 py-1 border-b border-base-300/20 flex flex-wrap items-center gap-1">
          {selectedObjectType && (
            <span
              className="badge badge-sm badge-accent gap-1 cursor-pointer hover:badge-error transition-colors"
              onClick={() => onSelectObjectType(null)}
              title={`Category: ${selectedObjectType} — click to clear`}
            >
              <span className="text-[10px]">{selectedObjectType}</span>
              <X size={10} />
            </span>
          )}
          {Object.entries(activeFilters)
            .filter(([, vals]) => vals.length > 0)
            .map(([key, vals]) => {
              const filterDef = categoryFilters.find((f) => f.key === key);
              return vals.map((val) => (
                <span
                  key={`${key}-${val}`}
                  className="badge badge-sm badge-primary gap-1 cursor-pointer hover:badge-error transition-colors"
                  onClick={() =>
                    onFilterChange(
                      key,
                      vals.filter((v) => v !== val),
                    )
                  }
                  title={`${filterDef?.label ?? key}: ${val} — click to remove`}
                >
                  <span className="text-[10px] max-w-20 truncate">{val}</span>
                  <X size={10} />
                </span>
              ));
            })}
          {statusFilter !== 'all' && (
            <span
              className={`badge badge-sm gap-1 cursor-pointer hover:badge-error transition-colors ${
                statusFilter === 'enabled' ? 'badge-success' : 'badge-warning'
              }`}
              onClick={() => onStatusFilterChange('all')}
              title={`Status: ${statusFilter} — click to clear`}
            >
              <span className="text-[10px]">
                {statusFilter.charAt(0).toUpperCase() + statusFilter.slice(1)}
              </span>
              <X size={10} />
            </span>
          )}
          <button
            className="text-[10px] text-error/60 hover:text-error ml-0.5 transition-colors"
            onClick={() => {
              onClearFilters();
              onSelectObjectType(null);
            }}
            title="Clear all filters"
          >
            Clear
          </button>
        </div>
      )}
    </>
  );
}
