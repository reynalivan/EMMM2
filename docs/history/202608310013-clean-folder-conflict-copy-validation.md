# Clean Folder Conflict Copy and Rename Validation

## Context

The Folder Conflict Manager header and footer repeated guidance already shown
inside the conflict list, while keeping the original name during a rename was
not rejected before submission.

## Changes

- Removed the duplicate header description and footer prefix note.
- Kept the conflict instructions in the candidate section and aligned the
  footer action to the right for a cleaner layout.
- Added an `unchanged` validation state for renamed candidates that retain
  their original base name.
- Added translated validation copy for English, Indonesian, and Chinese.

## Impacted Files

- `src/widgets/mod-explorer/modals/FolderConflictManager.tsx`
- `src/widgets/mod-explorer/modals/folderConflictValidation.ts`
- `src/widgets/mod-explorer/modals/folderConflictValidation.test.ts`
- `src/shared/i18n/locales/en/folder_grid.json`
- `src/shared/i18n/locales/id/folder_grid.json`
- `src/shared/i18n/locales/zh/folder_grid.json`

## Validation

- Focused Vitest suites: 22 tests passed.
- i18n lint passed.
- TypeScript check passed.
- Prettier check passed.
- ESLint passed with the existing `FolderConflictManager.tsx` max-lines warning.
- Production build passed.
