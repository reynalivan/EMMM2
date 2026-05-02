### Title

Refactor FolderGrid Selection to Explicit Checkbox Pattern

### Context

Previously, clicking anywhere on a `FolderCard` or `FolderListRow` inadvertently checked the item and threw the user into bulk selection mode, forcing the `BulkActionBar` to appear during simple item inspection. This was intrusive UX for a file-manager type interface.

### Changes

- Added `handleActivateItem` to `useFolderGridSelection` to allow setting an item as strictly active (open in preview) while simultaneously clearing the active bulk selection checkmarks.
- Rewired the standard `onClick` handler in both `FolderCard` and `FolderListRow` to call `onActivate` on standard left clicks instead of `toggleSelection`.
- The explicit checkbox overlay `<input type="checkbox">` and `Ctrl/Shift` modifier clicks remain bound to `toggleSelection` to properly enter bulk selection mode.
- Removed unused `clearSelection` prop parameters from the card components since clearing is now structurally handled by `handleActivateItem` logic inside the hook layer upstream.

### Impacted Files

- `src/features/folder-grid/hooks/useFolderGridSelection.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)

### Goal

Ensure that users can click to view a mod's details without the bottom Action Bar intruding, reserving bulk actions exclusively for deliberate checkbox/modifier interactions.

### Impact

- Eliminates annoying pop-up action bars for single-click inspection flows.
- Aligns grid navigation UX perfectly with default Windows Explorer / Google Drive behaviors.
