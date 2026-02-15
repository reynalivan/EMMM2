/**
 * Epic 3: CategorySection — collapsible category header with count badge.
 * Uses DaisyUI collapse + badge for a premium accordion UI.
 */

import { ChevronRight } from 'lucide-react';
import type { CategoryDef } from '../../types/object';
import { useAppStore } from '../../stores/useAppStore';

interface CategorySectionProps {
  category: CategoryDef;
  count: number;
  isSelected: boolean;
  onSelect: () => void;
  children: React.ReactNode;
}

export default function CategorySection({
  category,
  count,
  isSelected,
  onSelect,
  children,
}: CategorySectionProps) {
  const { collapsedCategories, toggleCategoryCollapse } = useAppStore();
  const isCollapsed = collapsedCategories.has(category.name);

  return (
    <div className="mb-1">
      {/* Category Header */}
      <div
        role="button"
        tabIndex={0}
        className={`flex items-center gap-2 px-3 py-2 rounded-lg cursor-pointer transition-all select-none group
          ${
            isSelected
              ? 'bg-primary/15 text-primary border border-primary/20'
              : 'hover:bg-base-200/50 text-base-content/60 hover:text-base-content/90'
          }`}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onSelect();
          }
        }}
      >
        {/* Expand/Collapse toggle */}
        <button
          className="p-0.5 rounded hover:bg-base-300/50 transition-colors"
          onClick={(e) => {
            e.stopPropagation();
            toggleCategoryCollapse(category.name);
          }}
          aria-label={`${isCollapsed ? 'Expand' : 'Collapse'} ${category.name}`}
        >
          <ChevronRight
            size={14}
            className={`transition-transform duration-200 ${isCollapsed ? '' : 'rotate-90'}`}
          />
        </button>

        {/* Category name */}
        <span className="text-xs font-semibold uppercase tracking-wider flex-1">
          {category.name}
        </span>

        {/* Count badge */}
        <span
          className={`badge badge-sm font-bold ${isSelected ? `badge-primary` : 'badge-ghost'}`}
        >
          {count}
        </span>
      </div>

      {/* Collapsible content */}
      {!isCollapsed && <div className="mt-0.5">{children}</div>}
    </div>
  );
}
