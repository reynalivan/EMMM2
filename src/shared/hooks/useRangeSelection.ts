import { useCallback, useMemo, useState } from 'react';

/**
 * useRangeSelection — shared anchor tracking + Shift-range computation for
 * list/grid multi-select. Callers own the selection set itself; this hook only
 * answers "which ids fall between the anchor and this target".
 */
export function useRangeSelection<T>(items: T[], getId: (item: T) => string) {
  const [anchorId, setAnchorId] = useState<string | null>(null);
  const ids = useMemo(() => items.map(getId), [items, getId]);

  /** Ids between the anchor and targetId (inclusive), or null when there is no valid range. */
  const getRange = useCallback(
    (targetId: string): string[] | null => {
      if (!anchorId) {
        return null;
      }

      const startIdx = ids.indexOf(anchorId);
      const endIdx = ids.indexOf(targetId);
      if (startIdx < 0 || endIdx < 0) {
        return null;
      }

      const [lo, hi] = startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
      return ids.slice(lo, hi + 1);
    },
    [anchorId, ids],
  );

  return { ids, anchorId, setAnchorId, getRange };
}
