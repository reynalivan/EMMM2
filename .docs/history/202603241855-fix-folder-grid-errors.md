# Fix FolderGrid errors and restore state

## Context

`FolderGrid.tsx` was experiencing compilation errors due to missing properties in the global store (`isIgnoreManagementOpen`) and the `useFolderGrid` hook (`duplicateWarning` and handlers).

## Changes

- Updated `useAppStore.ts` to include `isIgnoreManagementOpen` and `setIgnoreManagementOpen`.
- Updated `useFolderGridActions.ts` and `useFolderGrid.ts` to include/expose `duplicateWarning`, `handleDuplicateForceEnable`, `handleDuplicateEnableOnly`, and `handleDuplicateCancel`.
- Refactored `FolderGrid.tsx` to use reactive store selectors for ignore management and removed unused destructured variables that were causing lint warnings.

## Impacted Files

- `src/stores/useAppStore.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)

## Goal

`FolderGrid.tsx` now compiles correctly and follows project state management patterns.

## Impact

- Restored "Ignore Management" functionality in the Folder Grid.
- Improved build stability and resolved multiple TypeScript errors/warnings.
- No breaking changes.
