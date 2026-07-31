import { useState, useCallback, useMemo } from 'react';
import { useRangeSelection } from '../../../hooks/useRangeSelection';
import type { FlatItem } from './useObjectListVirtualizer';

type RowItem = Extract<FlatItem, { type: 'row' }>;

const getRowId = (item: RowItem) => item.obj.id;

/**
 * useObjectBulkSelect — manages multi-selection state for ObjectList rows.
 * Supports Ctrl+click (toggle), Shift+click (range), and clear.
 */
export function useObjectBulkSelect(flatItems: FlatItem[]) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const rowItems = useMemo(
    () => flatItems.filter((item): item is RowItem => item.type === 'row'),
    [flatItems],
  );
  const { ids: rowIds, setAnchorId, getRange } = useRangeSelection(rowItems, getRowId);

  const toggleSelection = useCallback(
    (id: string, isCtrl: boolean, isShift: boolean) => {
      setSelectedIds((prev) => {
        const next = new Set(prev);
        const range = isShift ? getRange(id) : null;

        if (range) {
          for (const rangeId of range) {
            next.add(rangeId);
          }
        } else if (isCtrl) {
          if (next.has(id)) next.delete(id);
          else next.add(id);
        } else {
          next.clear();
          next.add(id);
        }

        return next;
      });
      setAnchorId(id);
    },
    [getRange, setAnchorId],
  );

  const clearSelection = useCallback(() => {
    setSelectedIds(new Set());
    setAnchorId(null);
  }, [setAnchorId]);

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
