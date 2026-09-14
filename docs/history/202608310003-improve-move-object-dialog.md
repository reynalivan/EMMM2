# Improve Move to Object Dialog

## Context

The Move to Object dialog was rendered twice because both the folder grid and preview panel owned the same shared move-dialog state. The dialog was also too narrow, offered little context, and did not allow location search.

## Changes

- Made the folder grid the single owner of the shared Move to Object dialog.
- Redesigned the dialog with a wider two-panel layout for object and location selection.
- Added source selection summary, destination summary, object/location counts, loading states, and clearer post-move status guidance.
- Added client-side search for object names and destination locations.
- Reset search and selection state when a new move operation opens.
- Completed Move to Object copy in EN, ID, and ZH locales.
- Added coverage for location filtering and kept the dialog tests aligned with the new UI.

## Impacted Files

- `src/widgets/object-sidebar/modals/MoveToObjectDialog.tsx` (modified)
- `src/widgets/object-sidebar/modals/MoveToObjectDialogPanels.tsx` (added)
- `src/widgets/object-sidebar/modals/MoveToObjectDialog.test.tsx` (modified)
- `src/widgets/mod-explorer/modals/FolderGridModals.tsx` (existing single owner)
- `src/widgets/mod-preview/PreviewPanel.tsx` (modified)
- `src/widgets/mod-preview/components/PreviewPanelModals.tsx` (modified)
- `src/shared/i18n/locales/{en,id,zh}/folder_grid.json` (modified)

## Goal

Provide one clean, informative, searchable, and fully localized Move to Object experience for both single-item and bulk moves.

## Impact

The duplicate modal is removed without changing move business logic or backend payloads. Search remains local to the objects and destination locations already loaded by the existing queries.

## Notes

Validation completed with 23 targeted tests, `pnpm run build`, TypeScript, ESLint, i18n lint, and Prettier.
