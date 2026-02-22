import { useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';

// FolderGrid requires a calculated column count based on width
export function VirtualFolderGrid({ items }: { items: any[] }) {
  const parentRef = useRef<HTMLDivElement>(null);

  // Grid Constants
  const COLUMN_WIDTH = 200; // px
  const GAP = 16; // px

  // We assume a fixed container width for this example,
  // or use a ResizeObserver to update column count dynamically.
  const containerWidth = 800;
  const proceedColumns = Math.floor(containerWidth / (COLUMN_WIDTH + GAP));
  const rowCount = Math.ceil(items.length / proceedColumns);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 250, // Height of a generic card + gap
    overscan: 3,
    useFlushSync: false, // React 19 Fix
  });

  return (
    <div ref={parentRef} className="h-full w-full overflow-y-auto">
      <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          // Calculate items in this row
          const fromIndex = virtualRow.index * proceedColumns;
          const toIndex = Math.min(fromIndex + proceedColumns, items.length);
          const rowItems = items.slice(fromIndex, toIndex);

          return (
            <div
              key={virtualRow.index}
              className="absolute top-0 left-0 w-full flex gap-4 px-4"
              style={{
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              {rowItems.map((item) => (
                <div key={item.id} className="w-[200px] h-[230px] bg-base-200 rounded-lg p-4">
                  <img src={item.thumb} alt="" className="h-32 w-full object-cover rounded" />
                  <div className="mt-2 text-sm truncate">{item.name}</div>
                </div>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
