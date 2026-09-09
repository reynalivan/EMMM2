# Single Active Context Dialog

## Context

The Mark as Safe and Mark as Unsafe actions opened the same shared dialog in both the folder grid and Preview Panel. This produced duplicate dialogs, including one constrained inside the right preview panel.

## Changes

- Kept `ActiveModContextDialog` mounted only by `FolderGridModals`, which is the single owner for the shared active-context dialog.
- Removed the duplicate Preview Panel instance and its unused props.
- Centered the dialog with `modal-middle` and raised its stacking order above workspace panels.
- Added an accessible dialog title and reset the acknowledgement checkbox whenever a new context change is opened.

## Impacted Files

- `src/features/mod-runtime/modals/ActiveModContextDialog.tsx` (modified)
- `src/widgets/mod-explorer/modals/FolderGridModals.tsx` (modified)
- `src/widgets/mod-preview/PreviewPanel.tsx` (modified)
- `src/widgets/mod-preview/components/PreviewPanelModals.tsx` (modified)

## Goal

Ensure Mark as Safe and Mark as Unsafe always show one centered, consistent confirmation dialog regardless of the originating surface.

## Impact

The shared dialog state and context-change business logic remain unchanged. Only rendering ownership and presentation were adjusted.

## Notes

Validation completed with targeted Vitest tests, TypeScript, ESLint, Prettier, and production build.
