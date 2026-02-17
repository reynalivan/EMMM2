/**
 * Epic 3: FilterPanel — unified chip panel for category, sort, status, and metadata filters.
 * Absorbs CategorySelector and SortDropdown into a compact, responsive chip-based UI.
 * Covers: TC-3.4 (Schema-driven filtering)
 */

import { X } from 'lucide-react';
import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import type { FilterDef, CategoryDef } from '../../types/object';

interface FilterPanelProps {
  /** Metadata filters (Element, Weapon, Rarity, Gender) */
  filters: FilterDef[];
  activeFilters: Record<string, string[]>;
  onFilterChange: (key: string, values: string[]) => void;
  onClearAll: () => void;
  /** Status filter */
  statusFilter: 'all' | 'enabled' | 'disabled';
  onStatusFilterChange: (val: 'all' | 'enabled' | 'disabled') => void;
  /** Category filter (merged from CategorySelector) */
  categories?: CategoryDef[];
  selectedCategory: string | null;
  onSelectCategory: (type: string | null) => void;
  /** Sort (merged from SortDropdown) */
  sortBy: 'name' | 'date' | 'rarity';
  onSortChange: (val: 'name' | 'date' | 'rarity') => void;
}

const SORT_OPTIONS: { value: 'name' | 'date' | 'rarity'; label: string }[] = [
  { value: 'name', label: 'A–Z' },
  { value: 'date', label: 'New' },
  { value: 'rarity', label: '★' },
];

export default function FilterPanel({
  filters,
  activeFilters,
  onFilterChange,
  onClearAll,
  statusFilter,
  onStatusFilterChange,
  categories = [],
  selectedCategory,
  onSelectCategory,
  sortBy,
  onSortChange,
}: FilterPanelProps) {
  const [expandedFilter, setExpandedFilter] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const activeMetaCount = useMemo(
    () => Object.values(activeFilters).reduce((sum, arr) => sum + arr.length, 0),
    [activeFilters],
  );

  // Close dropdown when clicking outside
  const handleClickOutside = useCallback((e: MouseEvent) => {
    if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
      setExpandedFilter(null);
    }
  }, []);

  useEffect(() => {
    if (expandedFilter) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [expandedFilter, handleClickOutside]);

  return (
    <div ref={panelRef} className="px-3 py-2 border-b border-base-300/20 space-y-3">
      {/* Section 1: Sort */}
      <div className="flex items-center gap-2">
        <span className="text-[10px] uppercase font-bold text-base-content/30 tracking-wider">
          Sort
        </span>
        <div className="flex items-center gap-1">
          {SORT_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              className={`btn btn-xs rounded-md transition-all duration-150 ${
                sortBy === opt.value
                  ? 'btn-accent text-accent-content'
                  : 'btn-ghost border border-base-300/20 opacity-50 hover:opacity-100'
              }`}
              onClick={() => onSortChange(opt.value)}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      {/* Section 2: Filter */}
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <span className="text-[10px] uppercase font-bold text-base-content/30 tracking-wider">
            Filter
          </span>

          {/* Status chips */}
          <div className="flex items-center gap-1">
            {(['all', 'enabled', 'disabled'] as const).map((status) => (
              <button
                key={status}
                className={`btn btn-xs rounded-full transition-all duration-150 ${
                  statusFilter === status
                    ? status === 'enabled'
                      ? 'btn-success text-success-content'
                      : status === 'disabled'
                        ? 'btn-warning text-warning-content'
                        : 'btn-neutral text-neutral-content'
                    : 'btn-ghost opacity-50 hover:opacity-100'
                }`}
                onClick={() => onStatusFilterChange(status)}
              >
                {status.charAt(0).toUpperCase() + status.slice(1)}
              </button>
            ))}
          </div>
        </div>

        {/* Category chips */}
        {categories.length > 0 && (
          <div className="flex flex-wrap gap-1">
            <button
              className={`btn btn-xs rounded-full transition-all duration-150 ${
                !selectedCategory
                  ? 'btn-primary'
                  : 'btn-ghost border border-base-300/20 opacity-60 hover:opacity-100'
              }`}
              onClick={() => onSelectCategory(null)}
            >
              All Types
            </button>
            {categories.map((cat) => (
              <button
                key={cat.name}
                className={`btn btn-xs rounded-full transition-all duration-150 ${
                  selectedCategory === cat.name
                    ? 'btn-primary'
                    : 'btn-ghost border border-base-300/20 opacity-60 hover:opacity-100'
                }`}
                onClick={() => onSelectCategory(selectedCategory === cat.name ? null : cat.name)}
              >
                {cat.label ?? cat.name}
              </button>
            ))}
          </div>
        )}

        {/* Metadata filter chips (Element, Weapon, Rarity, Gender) */}
        {filters.length > 0 && (
          <div className="flex flex-wrap gap-1 pt-1">
            {filters.map((filter) => {
              const isExpanded = expandedFilter === filter.key;
              const selected = activeFilters[filter.key] ?? [];

              return (
                <div key={filter.key} className="relative">
                  <button
                    className={`btn btn-xs rounded-full gap-1 transition-all duration-150 ${
                      selected.length > 0
                        ? 'btn-primary'
                        : 'btn-ghost border border-base-300/30 opacity-70 hover:opacity-100'
                    }`}
                    onClick={() => setExpandedFilter(isExpanded ? null : filter.key)}
                  >
                    <span className="text-[11px]">{filter.label}</span>
                    {selected.length > 0 && (
                      <span className="badge badge-xs bg-base-100/20 text-inherit border-0">
                        {selected.length}
                      </span>
                    )}
                  </button>

                  {/* Dropdown options */}
                  {isExpanded && (
                    <div className="absolute top-full left-0 mt-1 z-50 p-1.5 rounded-lg bg-base-100/95 backdrop-blur-xl border border-white/10 shadow-xl min-w-40 max-h-52 overflow-y-auto">
                      {selected.length > 0 && (
                        <button
                          className="w-full text-left text-[10px] text-error/70 hover:text-error px-2 py-1 mb-0.5 transition-colors flex items-center gap-1"
                          onClick={() => onFilterChange(filter.key, [])}
                        >
                          <X size={10} />
                          Clear {filter.label}
                        </button>
                      )}
                      {filter.options.map((option) => {
                        const isActive = selected.includes(option);
                        return (
                          <label
                            key={option}
                            className={`flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors ${
                              isActive
                                ? 'bg-primary/10 hover:bg-primary/15'
                                : 'hover:bg-base-200/50'
                            }`}
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

            {/* Clear all metadata filters */}
            {activeMetaCount > 0 && (
              <button
                className="btn btn-xs btn-ghost rounded-full gap-1 text-error/70 hover:text-error"
                onClick={onClearAll}
              >
                <X size={10} />
                <span className="text-[11px]">Clear</span>
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
