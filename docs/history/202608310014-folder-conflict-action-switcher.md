# Folder Conflict Action Switcher

## Context

The conflict card exposed Rename and Trash as separate actions, and Trash was
executed immediately through a confirmation dialog. The final action button
also always described a rename operation even when the reviewed plan differed.

## Changes

- Replaced the per-card Trash action with a Rename / Mark as Trash switcher.
- Keep Trash selections local until the yellow Apply button is pressed.
- Apply pending Trash selections first, then submit the remaining rename plan.
- Make the final action label reflect rename-only, Trash-only, mixed, and empty
  plans, with a Next indicator only when another conflict group remains.
- Persist action choices alongside rename drafts while navigating or refreshing
  conflict groups.
- Validate rename drafts on blur so empty and duplicate names are visible before
  the final Apply action.
- Removed the obsolete Trash confirmation component and localized the new copy
  for English, Indonesian, and Chinese.

## Impacted Files

- `src/widgets/mod-explorer/modals/FolderConflictManager.tsx`
- `src/widgets/mod-explorer/modals/FolderConflictCandidateCard.tsx`
- `src/widgets/mod-explorer/modals/folderConflictDrafts.ts`
- `src/widgets/mod-explorer/modals/folderConflictDrafts.test.ts`
- `src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx`
- `src/widgets/mod-explorer/modals/FolderConflictManager.trash.test.tsx`
- `src/shared/i18n/locales/en/folder_grid.json`
- `src/shared/i18n/locales/id/folder_grid.json`
- `src/shared/i18n/locales/zh/folder_grid.json`
- `src/widgets/mod-explorer/modals/FolderConflictTrashDialog.tsx`

## Validation

- Focused Vitest suites: 12 tests passed.
- i18n lint passed.
- TypeScript check passed.
