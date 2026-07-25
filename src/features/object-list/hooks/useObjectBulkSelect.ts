import { useState, useCallback, useMemo } from 'react';
import type { FlatItem } from './useObjectListVirtualizer';

/**
 * useObjectBulkSelect — manages multi-selection state for ObjectList rows.
 * Supports Ctrl+click (toggle), Shift+click (range), and clear.
 */
export function useObjectBulkSelect(flatItems: FlatItem[]) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  /** Last toggled item (anchor for Shift-range) */
  const lastToggledId = useState<string | null>(null);

  /** All row-type item IDs in flat order (for range select) */
  const rowIds = useMemo(
    () =>
      flatItems
        .filter((item): item is Extract<FlatItem, { type: 'row' }> => item.type === 'row')
        .map((item) => item.obj.id),
    [flatItems],
  );

  const toggleSelection = useCallback(
    (id: string, isCtrl: boolean, isShift: boolean) => {
      setSelectedIds((prev) => {
        const next = new Set(prev);

        if (isShift && lastToggledId[0]) {
          // Range select: from lastToggled to id
          const startIdx = rowIds.indexOf(lastToggledId[0]);
          const endIdx = rowIds.indexOf(id);
          if (startIdx >= 0 && endIdx >= 0) {
            const [lo, hi] = startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
            for (let i = lo; i <= hi; i++) {
              next.add(rowIds[i]);
            }
          }
        } else if (isCtrl) {
          // Toggle single
          if (next.has(id)) next.delete(id);
          else next.add(id);
        } else {
          // Plain click: clear + select single
          next.clear();
          next.add(id);
        }

        return next;
      });
      lastToggledId[1](id);
    },
    [rowIds, lastToggledId],
  );

  const clearSelection = useCallback(() => {
    setSelectedIds(new Set());
    lastToggledId[1](null);
  }, [lastToggledId]);

  const selectAll = useCallback(() => {
    setSelectedIds(new Set(rowIds));
  }, [rowIds]);

  const isSelected = useCallback((id: string) => selectedIds.has(id), [selectedIds]);

  return {
    selectedIds,
    selectionCount: selectedIds.size,
    isAnySelected: selectedIds.size > 0,
    toggleSelection,
    clearSelection,
    selectAll,
    isSelected,
  };
}
