## Fix Bulk Metadata Dialog Overlay

### Context

When selecting multiple folders and clicking "Edit Metadata", the dialog overlay was blurred but the content was missing. This was caused by an improper modal structure that didn't comply with daisyUI 5 requirements.

### Changes

- Refactored `BulkTagModal` to use the standard `<dialog className="modal">` element.
- Replaced custom portal and `fixed` overlay with daisyUI's built-in modal logic.
- Added `modal-backdrop` for "click-to-close" functionality.
- Optimized Tailwind classes and removed unused `createPortal` import.

### Impacted Files

- `src/features/folder-grid/BulkTagModal.tsx` (modified)

### Goal

Restore visibility and standard behavior to the bulk metadata dialog in Folder Grid.

### Impact

- Fixed the UI bug preventing bulk tag editing.
- Improved consistency with standard project modal patterns.
- No breaking changes.
