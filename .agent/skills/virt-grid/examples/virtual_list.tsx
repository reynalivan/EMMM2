import { useRef, useEffect } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useInfiniteQuery } from '@tanstack/react-query';

export function VirtualLogList() {
  const parentRef = useRef<HTMLDivElement>(null);

  // 1. Data Fetching
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage } = useInfiniteQuery({
    queryKey: ['logs'],
    queryFn: fetchLogs,
    getNextPageParam: (lastPage) => lastPage.nextCursor,
    initialPageParam: 0,
  });

  // Flatten pages
  const rows = data ? data.pages.flatMap((page) => page.rows) : [];

  // 2. Virtualizer Setup
  const rowVirtualizer = useVirtualizer({
    count: hasNextPage ? rows.length + 1 : rows.length, // +1 for loading spinner
    getScrollElement: () => parentRef.current,
    estimateSize: () => 35, // 35px row height
    overscan: 5,
    useFlushSync: false, // React 19 Fix
  });

  const virtualItems = rowVirtualizer.getVirtualItems();

  // 3. Infinite Scroll Trigger
  useEffect(() => {
    const [lastItem] = [...virtualItems].reverse();
    if (!lastItem) return;

    if (lastItem.index >= rows.length - 1 && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [hasNextPage, fetchNextPage, rows.length, isFetchingNextPage, virtualItems]);

  return (
    // Container: Must have height + overflow
    <div ref={parentRef} className="h-[500px] w-full overflow-y-auto contain-strict">
      <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
        {virtualItems.map((virtualRow) => {
          const isLoaderRow = virtualRow.index > rows.length - 1;
          const post = rows[virtualRow.index];

          return (
            <div
              key={virtualRow.index}
              className="absolute top-0 left-0 w-full"
              style={{
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              {isLoaderRow ? (
                <span>Loading more...</span>
              ) : (
                <div className="p-2 border-b border-base-300">{post.message}</div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// Mock Fetcher
async function fetchLogs({ pageParam }: { pageParam: number }) {
  // ... implementation
  return { rows: [], nextCursor: pageParam + 1 };
}
