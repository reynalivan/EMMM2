/**
 * useFolderGridLayout — Virtualization, layout math, and scroll persistence.
 *
 * Extracted from useFolderGrid to keep the orchestrator under 350 lines.
 */

import { useState, useEffect } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';

// Grid layout constants
const CARD_MIN_W = 160;
const CARD_MAX_W = 280;
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
    ? Math.min(CARD_MAX_W, Math.floor((containerWidth - GAP * (columnCount - 1)) / columnCount))
    : 0;
  const cardHeight = isGridView ? Math.round(cardWidth * (4 / 3)) + CARD_INFO_H : 0;
  const rowCount = isGridView ? Math.ceil(itemCount / columnCount) : itemCount;

  // ── ResizeObserver ────────────────────────────────────────────────────────
  // parentRef is a stable ref — intentionally omitted from deps
  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) setContainerWidth(entry.contentRect.width);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Virtualizer ───────────────────────────────────────────────────────────
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => (isGridView ? cardHeight + GAP : LIST_ROW_HEIGHT),
    overscan: 5,
    initialOffset: explorerScrollOffset,
  });

  // Reset scroll on sub-path change
  useEffect(() => {
    rowVirtualizer.scrollToOffset(0);
  }, [explorerSubPath]); // eslint-disable-line react-hooks/exhaustive-deps

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
  }, [setExplorerScrollOffset]); // eslint-disable-line react-hooks/exhaustive-deps

  return { rowVirtualizer, columnCount, cardWidth };
}
