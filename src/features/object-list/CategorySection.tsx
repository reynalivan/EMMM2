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
      className={`flex items-center gap-2 px-3 py-1 mt-2 mb-1 select-none cursor-pointer hover:bg-base-content/5 transition-colors ${isSelected ? 'bg-base-content/10' : ''}`}
      onClick={onSelect}
    >
      {/* Label + count */}
      <span
        className={`text-[10px] font-bold uppercase tracking-widest ${isSelected ? 'text-primary' : 'text-base-content/40'}`}
      >
        {category.label ?? category.name}
      </span>
      <span className="text-[10px] tabular-nums font-bold text-base-content/25">{count}</span>

      {/* Right line */}
      <div className={`flex-1 h-px ${isSelected ? 'bg-primary/20' : 'bg-base-content/5'}`} />
    </div>
  );
}
