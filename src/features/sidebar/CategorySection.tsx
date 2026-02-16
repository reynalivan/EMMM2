/**
 * Epic 3: CategorySection — non-collapsible category divider with label and count.
 * Simple horizontal line + centered label + count badge. Click = filter by category.
 */

import type { CategoryDef } from '../../types/object';

interface CategorySectionProps {
  category: CategoryDef;
  count: number;
  isSelected: boolean;
  onSelect: () => void;
}

export default function CategorySection({
  category,
  count,
  isSelected,
  onSelect,
}: CategorySectionProps) {
  return (
    <div
      role="button"
      tabIndex={0}
      className={`flex items-center gap-2 px-3 py-1 cursor-pointer select-none group transition-colors duration-150
        ${isSelected ? 'text-primary' : 'text-base-content/40 hover:text-base-content/60'}`}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect();
        }
      }}
    >
      {/* Left line */}
      <div
        className={`flex-1 h-px transition-colors ${isSelected ? 'bg-primary/30' : 'bg-base-content/10 group-hover:bg-base-content/15'}`}
      />

      {/* Label + count */}
      <span className="text-[10px] font-semibold uppercase tracking-widest whitespace-nowrap">
        {category.label ?? category.name}
      </span>
      <span
        className={`text-[10px] tabular-nums font-bold ${isSelected ? 'text-primary/70' : 'text-base-content/25'}`}
      >
        {count}
      </span>

      {/* Right line */}
      <div
        className={`flex-1 h-px transition-colors ${isSelected ? 'bg-primary/30' : 'bg-base-content/10 group-hover:bg-base-content/15'}`}
      />
    </div>
  );
}
