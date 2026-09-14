# Disable Invalid Folder Conflict Actions

## Context

The conflict dialog could still submit its final action while a rename field
was showing a validation error.

## Changes

- Disable the dynamic Apply action while any rename validation error remains.
- Keep the Rename / Mark as Trash switcher available so the user can correct
  the name or choose another valid action.
- Re-enable Apply automatically after the current rename plan validates.

## Impacted Files

- `src/widgets/mod-explorer/modals/FolderConflictManager.tsx`
- `src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx`

## Validation

- Folder conflict manager tests passed.
