### Title

Fix Grid Resize Tearing and Increase Minimum Mod Width

### Context

After migrating to CSS Grid `1fr` logic, a new issue exposed itself during active window resizing. Because native CSS `1fr` recalculations happen instantly but Javascript TanStack Virtualizer calculates absolute row heights via ResizeObserver limits at a slower tick rate, the grid cards experienced "Asynchronous Tearing." Cards would auto-expand their aspect-square images faster than their CSS wrappers, causing them to briefly overlap or leave jagged vertical gaps when actively dragging the window. Additionally, `160px` minimum width was considered too microscopic for modern UX.

### Changes

- Escaped the CSS fractional `1fr` trap by reverting Virtualized `gridTemplateColumns` to strict Javascript absolute width anchoring: `repeat(${columnCount}, ${cardWidth}px)`. 
- Placed `justify-center` onto the virtual `FolderGrid` row class string to gracefully distribute any millimeter differences uniformly at the edges.
- Scaled `CARD_MIN_W` (Minimum card width specification) significantly from `160` to `210` to bring cover-art display logic to contemporary application aesthetics.
- Added `cardWidth` property out of `useFolderGrid.ts` root destructuring.

### Impacted Files

- `src/features/folder-grid/hooks/useFolderGridLayout.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)

### Goal

Secure 100% frame-perfect lockstep alignment between CSS-drawn elements inside the viewport and React-Virtualizer memory-rendered pixel boundaries, while significantly upgrading the size presence of mods on standard displays.

### Impact

- Window resize calculations are bullet-proofed; card aspect ratios scale flawlessly parallel to virtual container bounds ensuring zero grid overlaps.
- Default scaling factor presents fewer, significantly larger cards horizontally, resolving squished layout behaviors.
