/**
 * useFolderGridLayout — Virtualization, layout math, and scroll persistence.
 *
 * Extracted from useFolderGrid to keep the orchestrator under 350 lines.
 */

'use no memo';

import { useState, useEffect, useCallback } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';

// Grid layout constants
const CARD_MIN_W = 180;
const CARD_INFO_H = 70;
const LIST_ROW_HEIGHT = 52;
const GAP = 12;

interface FolderGridLayoutOptions {
  parentRef: React.RefObject<HTMLDivElement | null>;
  explorerSubPath: string | undefined;
  explorerScrollOffset: number;
  setExplorerScrollOffset: (v: number) => void;
  isGridView: boolean;
  itemCount: number;
}

export function useFolderGridLayout({
  parentRef,
  explorerSubPath,
  explorerScrollOffset,
  setExplorerScrollOffset,
  isGridView,
  itemCount,
}: FolderGridLayoutOptions) {
  const [containerWidth, setContainerWidth] = useState(800);

  // ── Grid dimension math ───────────────────────────────────────────────────
  const columnCount = isGridView
    ? Math.max(1, Math.floor((containerWidth + GAP) / (CARD_MIN_W + GAP)))
    : 1;
  const cardWidth = isGridView
    ? Math.floor((containerWidth - GAP * (columnCount - 1)) / columnCount)
    : 0;
  // Actual image container uses aspect-square (1:1), so height equals cardWidth
  const cardHeight = isGridView ? Math.round(cardWidth) + CARD_INFO_H : 0;
  const rowCount = isGridView ? Math.ceil(itemCount / columnCount) : itemCount;

  // ── ResizeObserver ────────────────────────────────────────────────────────
  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) setContainerWidth(entry.contentRect.width);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [parentRef]);

  // ── Virtualizer ───────────────────────────────────────────────────────────
  /**
   * TanStack Virtual's useVirtualizer is technically incompatible with React Compiler
   * because it returns an object with methods that break auto-memoization.
   * We isolate it here and return a stable, pure API to the orchestrator.
   */
  // eslint-disable-next-line react-hooks/incompatible-library
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => (isGridView ? cardHeight + GAP : LIST_ROW_HEIGHT),
    overscan: 5,
    initialOffset: explorerScrollOffset,
  });

  // Force virtualizer to recalculate when row height changes to prevent tearing/overlaps
  useEffect(() => {
    rowVirtualizer.measure();
  }, [cardHeight, isGridView, rowVirtualizer]);

  const scrollToIndex = useCallback(
    (index: number, options?: { align?: 'start' | 'center' | 'end' | 'auto' }) => {
      rowVirtualizer.scrollToIndex(index, options);
    },
    [rowVirtualizer],
  );

  const scrollToOffset = useCallback(
    (offset: number, options?: { align?: 'start' | 'center' | 'end' | 'auto' }) => {
      rowVirtualizer.scrollToOffset(offset, options);
    },
    [rowVirtualizer],
  );

  // Reset scroll on sub-path change
  useEffect(() => {
    scrollToOffset(0);
  }, [explorerSubPath, scrollToOffset]);

  // Persist scroll offset
  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    let rafId: number;
    const handleScroll = () => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => setExplorerScrollOffset(el.scrollTop));
    };
    el.addEventListener('scroll', handleScroll, { passive: true });
    return () => {
      el.removeEventListener('scroll', handleScroll);
      cancelAnimationFrame(rafId);
    };
  }, [parentRef, setExplorerScrollOffset]);

  return {
    virtualItems: rowVirtualizer.getVirtualItems(),
    totalSize: rowVirtualizer.getTotalSize(),
    scrollToIndex,
    scrollToOffset,
    columnCount,
    cardWidth,
  };
}
