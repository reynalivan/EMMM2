# Folder Conflict Queue Resolution

## Context

Resolving the first folder-name conflict could make the manager show every
conflict as resolved when a reconcile result briefly returned an empty list.

## Changes

- Treat an empty multi-group report as ambiguous instead of resolving every
  previous group.
- Track the exact `group_id` completed by an explicit rename or Trash action.
- Preserve remaining conflict groups when an explicit action returns `Applied`
  with an empty conflict list.
- Keep the queue total stable while a conflict report is refreshing.
- Add regression coverage for the two-group failure and the explicit final
  resolution path.

## Impacted Files

- `src/widgets/mod-explorer/modals/folderConflictQueue.ts`
- `src/widgets/mod-explorer/modals/folderConflictQueue.test.ts`
- `src/widgets/mod-explorer/modals/useApplyFolderConflictActionResult.ts`
- `src/widgets/mod-explorer/modals/FolderConflictManager.tsx`
- `src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx`

## Validation

- Focused Vitest suites: 40 tests passed.
- TypeScript check passed.
- ESLint passed with the existing `FolderConflictManager.tsx` max-lines warning.
- Prettier check passed.
- Production build passed.
