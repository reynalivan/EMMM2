/**
 * Epic 3: FilterPanel — unified chip panel for category, sort, status, and metadata filters.
 * Absorbs CategorySelector and SortDropdown into a compact, responsive chip-based UI.
 * Covers: TC-3.4 (Schema-driven filtering)
 */

import { X } from 'lucide-react';
import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { FilterDef, CategoryDef } from '@/entities/game-object';
import { LiquidSurface } from '@/shared/ui/liquid';

const QUIET_CHIP_BASE =
  'btn btn-xs border bg-base-200 transition-[background-color,border-color,color] duration-150';
const QUIET_CHIP_IDLE =
  'border-base-content/10 text-base-content/60 hover:border-base-content/20 hover:bg-base-300 hover:text-base-content';
const QUIET_CHIP_ACTIVE = {
  neutral:
    'border-base-content/20 text-base-content hover:border-base-content/30 hover:bg-base-300',
  primary: 'border-primary/25 text-primary hover:border-primary/35 hover:bg-base-300',
  success: 'border-success/25 text-success hover:border-success/35 hover:bg-base-300',
  warning: 'border-warning/25 text-warning hover:border-warning/35 hover:bg-base-300',
} as const;

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
  const { t } = useTranslation(['objects']);
  const [expandedFilter, setExpandedFilter] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const SORT_OPTIONS: { value: 'name' | 'date' | 'rarity'; label: string }[] = useMemo(
    () => [
      { value: 'name', label: t('filter.sort_az') },
      { value: 'date', label: t('filter.sort_new') },
      { value: 'rarity', label: t('filter.sort_rarity') },
    ],
    [t],
  );

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
          {t('filter.sort_label')}
        </span>
        <div className="flex items-center gap-1">
          {SORT_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              className={`${QUIET_CHIP_BASE} rounded-md ${
                sortBy === opt.value ? QUIET_CHIP_ACTIVE.primary : QUIET_CHIP_IDLE
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
            {t('filter.filter_label')}
          </span>

          {/* Status chips */}
          <div className="flex items-center gap-1">
            {(['all', 'enabled', 'disabled'] as const).map((status) => (
              <button
                key={status}
                className={`${QUIET_CHIP_BASE} rounded-full ${
                  statusFilter === status
                    ? status === 'enabled'
                      ? QUIET_CHIP_ACTIVE.success
                      : status === 'disabled'
                        ? QUIET_CHIP_ACTIVE.warning
                        : QUIET_CHIP_ACTIVE.neutral
                    : QUIET_CHIP_IDLE
                }`}
                onClick={() => onStatusFilterChange(status)}
              >
                {status === 'all'
                  ? t('filter.status_all')
                  : status === 'enabled'
                    ? t('filter.status_active')
                    : t('filter.status_disabled')}
              </button>
            ))}
          </div>
        </div>

        {/* Category chips */}
        {categories.length > 0 && (
          <div className="flex flex-wrap gap-1">
            <button
              className={`${QUIET_CHIP_BASE} rounded-full ${
                !selectedCategory ? QUIET_CHIP_ACTIVE.primary : QUIET_CHIP_IDLE
              }`}
              onClick={() => onSelectCategory(null)}
            >
              {t('filter.all_types')}
            </button>
            {categories.map((cat) => (
              <button
                key={cat.name}
                className={`${QUIET_CHIP_BASE} rounded-full ${
                  selectedCategory === cat.name ? QUIET_CHIP_ACTIVE.primary : QUIET_CHIP_IDLE
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
                    className={`${QUIET_CHIP_BASE} rounded-full gap-1 ${
                      selected.length > 0 ? QUIET_CHIP_ACTIVE.primary : QUIET_CHIP_IDLE
                    }`}
                    onClick={() => setExpandedFilter(isExpanded ? null : filter.key)}
                  >
                    <span className="text-[11px]">{filter.label}</span>
                    {selected.length > 0 && (
                      <span className="badge badge-xs border-0 bg-base-content/10 text-inherit">
                        {selected.length}
                      </span>
                    )}
                  </button>

                  {/* Dropdown options */}
                  {isExpanded && (
                    <LiquidSurface
                      liquidRole="overlay"
                      className="absolute left-0 top-full z-[var(--workspace-layer-popover)] mt-1 w-56 max-w-[calc(100vw-2rem)] max-h-52 rounded-lg shadow-xl"
                    >
                      <div className="max-h-52 overflow-y-auto p-1.5">
                        {selected.length > 0 && (
                          <button
                            className="w-full text-left text-[10px] text-error/70 hover:text-error px-2 py-1 mb-0.5 transition-colors flex items-center gap-1"
                            onClick={() => onFilterChange(filter.key, [])}
                          >
                            <X size={10} />
                            {t('filter.clear_filter', { label: filter.label })}
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
                    </LiquidSurface>
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
                <span className="text-[11px]">{t('filter.clear_meta')}</span>
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
