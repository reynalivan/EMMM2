# Implement FolderGrid Bulk Select Checkboxes

## Context

Bulk action overlay existed in FolderGrid, but individual items lacked checkboxes for easy multi-selection, forcing users to rely on Ctrl/Shift-Clicking which was less discoverable and harder to use.

## Changes

- Added multi-select checkbox overlay to `FolderCard` (grid view) and `FolderListRow` (list view) thumbnail areas.
- Updated `FolderListRow` prop types to include `isShift` parameter for range selection.
- Fixed `FolderGrid.tsx` range selection logic in list mode by forwarding the `isShift` flag.
- Ensured checkboxes are visible on hover or when selected, consistent with `ObjectRowItem`.
- **Gap Fixes**: Added `Escape` shortcut to clear selection and `Delete` shortcut to trigger bulk deletion.
- **Performance**: Optimized "Select All" (`Ctrl+A` and right-click menu) to use a single `setGridSelection` operation instead of looping over React state updates.

## Impacted Files

- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)

## Goal

Improve usability of FolderGrid bulk actions by providing a clear, interactive visual selection mechanism (checkboxes), fixing range selection in list view, and ensuring full keyboard sequence parity with ObjectList.

## Impact

- Better UX for bulk operations.
- Shift-click range selection now fully functional in list view.
- Consistent UI and keyboard shortcuts (`Escape`, `Delete`, `Ctrl+A`) with ObjectList component.
- Improved performance for "Select All" operations.
