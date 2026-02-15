/**
 * Epic 3: FilterPanel — schema-driven filter UI for sidebar.
 * Renders dynamic filter chips based on game schema.
 * Covers: TC-3.4 (Schema-driven filtering)
 */

import { Filter, X } from 'lucide-react';
import { useState, useMemo } from 'react';
import type { FilterDef } from '../../types/object';

interface FilterPanelProps {
  filters: FilterDef[];
  activeFilters: Record<string, string[]>;
  onFilterChange: (key: string, values: string[]) => void;
  onClearAll: () => void;
}

export default function FilterPanel({
  filters,
  activeFilters,
  onFilterChange,
  onClearAll,
}: FilterPanelProps) {
  const [expandedFilter, setExpandedFilter] = useState<string | null>(null);

  const activeCount = useMemo(
    () => Object.values(activeFilters).reduce((sum, arr) => sum + arr.length, 0),
    [activeFilters],
  );

  if (filters.length === 0) return null;

  return (
    <div className="px-2 pb-2">
      {/* Filter header */}
      <div className="flex items-center justify-between px-1 py-1.5">
        <div className="flex items-center gap-1.5 text-base-content/50">
          <Filter size={12} />
          <span className="text-xs font-medium uppercase tracking-wider">Filters</span>
          {activeCount > 0 && <span className="badge badge-xs badge-primary">{activeCount}</span>}
        </div>
        {activeCount > 0 && (
          <button
            className="text-xs text-error/70 hover:text-error transition-colors flex items-center gap-0.5"
            onClick={onClearAll}
          >
            <X size={10} />
            Clear
          </button>
        )}
      </div>

      {/* Filter groups */}
      <div className="flex flex-wrap gap-1.5 px-1">
        {filters.map((filter) => {
          const isExpanded = expandedFilter === filter.key;
          const selected = activeFilters[filter.key] ?? [];

          return (
            <div key={filter.key} className="relative">
              {/* Filter toggle button */}
              <button
                className={`btn btn-xs rounded-full gap-1 ${
                  selected.length > 0
                    ? 'btn-primary btn-outline'
                    : 'btn-ghost opacity-60 hover:opacity-100'
                }`}
                onClick={() => setExpandedFilter(isExpanded ? null : filter.key)}
              >
                {filter.label}
                {selected.length > 0 && (
                  <span className="badge badge-xs badge-primary">{selected.length}</span>
                )}
              </button>

              {/* Dropdown options */}
              {isExpanded && (
                <div className="absolute top-full left-0 mt-1 z-50 p-2 rounded-lg bg-base-100/95 backdrop-blur-xl border border-white/10 shadow-xl min-w-36 max-h-48 overflow-y-auto">
                  {filter.options.map((option) => {
                    const isActive = selected.includes(option);
                    return (
                      <label
                        key={option}
                        className="flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer hover:bg-base-200/50 transition-colors"
                      >
                        <input
                          type="checkbox"
                          className="checkbox checkbox-xs checkbox-primary"
                          checked={isActive}
                          onChange={() => {
                            const next = isActive
                              ? selected.filter((v) => v !== option)
                              : [...selected, option];
                            onFilterChange(filter.key, next);
                          }}
                        />
                        <span className="text-xs text-base-content/80">{option}</span>
                      </label>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
