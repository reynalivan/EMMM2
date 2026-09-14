# Auto-Rename Folder Conflicts

## Context

Folder conflict cards initially reused the same base name for every candidate, which made the rename plan ambiguous. The existing Move to Trash action also needed to remain a deliberate, confirmed action.

## Changes

- Keep the selected folder's base name and automatically generate unique names such as `2.Jacket-02` and `2.Jacket-03` for the remaining duplicates.
- Recalculate generated names when the user changes which folder keeps the base name, while preserving manually edited drafts.
- Explain the automatic naming behavior in the English, Indonesian, and Chinese conflict-manager locales.
- Preserve and verify the centered Move to Trash confirmation dialog and its keyboard behavior.

## Impacted Files

- `src/widgets/mod-explorer/modals/folderConflictDrafts.ts` (modified)
- `src/widgets/mod-explorer/modals/folderConflictDrafts.test.ts` (added)
- `src/widgets/mod-explorer/modals/FolderConflictManager.tsx` (modified)
- `src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx` (modified)
- `src/shared/i18n/locales/{en,id,zh}/folder_grid.json` (modified)

## Goal

Make duplicate folder resolution predictable, collision-free by default, and safe to confirm.

## Impact

Generated rename drafts are unique within each conflict group. User-entered names remain unchanged unless they are still an untouched default that must be regenerated after changing the kept folder.

## Notes

Validation completed with focused conflict-manager tests, TypeScript, ESLint, Prettier, i18n lint, and production build.
