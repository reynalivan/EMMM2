# Virtualization Best Practices (React 19)

## 1. React 19 Specifics
### `flushSync` Warning
React 19 batches updates differently. TanStack Virtual v3 may warn about `flushSync`.
**Fix:**
```tsx
const rowVirtualizer = useVirtualizer({
  // ...
  useFlushSync: false, // Critical for React 19
});
```

## 2. Dynamic Heights
For items with variable content (Chat logs, wrapping text):
1.  **Estimate**: Provide a `estimateSize` closest to the average.
2.  **Measure**: Use `measureElement` ref on the item.
```tsx
<div
  ref={rowVirtualizer.measureElement} // Auto-measures height
  data-index={index}
>
  {content}
</div>
```

## 3. Overscan
-   **Purpose**: Renders items outside the viewport to prevent "white flash" during fast scrolling.
-   **Value**: `overscan: 5` is usually sufficient. Increase to `10-20` for heavy images.

## 4. Scroll Restoration
If navigating away and back, you must save the scroll offset.
-   **Simple**: Use `initialOffset` option.
-   **Advanced**: Sync `virtualizer.scrollOffset` to global state (Zustand).

## 5. Infinite Scroll Logic
Do **NOT** use `IntersectionObserver` on a sentinel element.
**Correct Way**: Check the *last virtual item index*.

```tsx
const [lastItem] = [...rowVirtualizer.getVirtualItems()].reverse();

useEffect(() => {
    if (!lastItem) return;

    if (
        lastItem.index >= allRows.length - 1 &&
        hasNextPage &&
        !isFetchingNextPage
    ) {
        fetchNextPage();
    }
}, [
    hasNextPage,
    fetchNextPage,
    allRows.length,
    isFetchingNextPage,
    rowVirtualizer.getVirtualItems(),
]);
```
