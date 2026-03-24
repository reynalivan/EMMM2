# React Compiler Optimization: Folder Grid Virtualizer

### Context

Resolving `react-hooks/incompatible-library` warning in `FolderGrid` feature which was suppressing React Compiler memoization across the entire feature.

### Changes

- Refactored `useFolderGridLayout` to isolate the incompatible TanStack Virtual `useVirtualizer` hook.
- Replaced raw `Virtualizer` instance return with a stable, pure API (`virtualItems`, `totalSize`, `scrollToIndex`, `scrollToOffset`).
- Removed `'use no memo';` from the `useFolderGrid` orchestrator hook, allowing full React Compiler optimization.
- Updated `FolderGrid` component to consume discrete properties from the orchestrator.

### Impacted Files

- `src/features/folder-grid/hooks/useFolderGridLayout.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)

### Goal

Isolate incompatible third-party libraries at the boundary while maintaining full React Compiler benefits for the application's core logic.

### Impact

- Significant improvement in UI performance for large folder grids (1000+ items) due to restored auto-memoization.
- Cleaner component interface by hiding external library instances from the rendering layer.
