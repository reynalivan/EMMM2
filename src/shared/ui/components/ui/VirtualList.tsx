import { useCallback, useMemo, useState, type ReactNode } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { cn } from '@/shared/lib/utils';

interface VirtualListProps<T> {
  items: readonly T[];
  getItemKey: (item: T) => string;
  estimateSize: (item: T) => number;
  renderItem: (item: T) => ReactNode;
  ariaLabel: string;
  className?: string;
  contentClassName?: string;
  overscan?: number;
  initialOffset?: number | (() => number);
  onScrollOffsetChange?: (offset: number) => void;
}

export default function VirtualList<T>({
  items,
  getItemKey,
  estimateSize,
  renderItem,
  ariaLabel,
  className,
  contentClassName,
  overscan = 6,
  initialOffset,
  onScrollOffsetChange,
}: VirtualListProps<T>) {
  const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
  const [resolvedInitialOffset] = useState(() =>
    typeof initialOffset === 'function' ? initialOffset() : (initialOffset ?? 0),
  );
  const setScrollRef = useCallback((element: HTMLDivElement | null) => {
    setScrollElement(element);
  }, []);
  const itemKeys = useMemo(() => items.map(getItemKey), [items, getItemKey]);
  const getVirtualItemKey = useCallback((index: number) => itemKeys[index]!, [itemKeys]);

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollElement,
    estimateSize: (index) => estimateSize(items[index]!),
    getItemKey: getVirtualItemKey,
    overscan,
    initialOffset: resolvedInitialOffset,
    initialRect: { width: 0, height: 1_000 },
  });

  return (
    <div
      ref={setScrollRef}
      className={cn('min-h-0 flex-1 overflow-y-auto overscroll-contain', className)}
      role="list"
      aria-label={ariaLabel}
      onScroll={(event) => onScrollOffsetChange?.(event.currentTarget.scrollTop)}
    >
      <div
        className={cn('relative w-full', contentClassName)}
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const item = items[virtualItem.index];
          if (!item) return null;

          return (
            <div
              key={virtualItem.key}
              ref={virtualizer.measureElement}
              data-index={virtualItem.index}
              role="listitem"
              className="absolute left-0 top-0 w-full"
              style={{ transform: `translateY(${virtualItem.start}px)` }}
            >
              {renderItem(item)}
            </div>
          );
        })}
      </div>
    </div>
  );
}
