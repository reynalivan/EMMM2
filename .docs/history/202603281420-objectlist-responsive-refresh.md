## Title

ObjectList responsive refresh and row repaint hardening

## Context

ObjectList counts, disabled state, names, and thumbnails were lagging behind local mutations and watcher updates because several flows only marked object queries stale, some invalidated the wrong count key, and row memoization skipped visual props.

## Changes

- Added a shared `refreshObjectListQueries()` helper in the object query layer so ObjectList rows and object count queries refresh together.
- Switched mod, folder, collection, corridor, scanner, conflict-resolution, and move/import flows from ad-hoc `['objects']` or `['category-counts']` invalidation to active ObjectList refresh.
- Updated disk reconcile to refresh ObjectList when `folders_changed`, `objects_changed`, or `path_updates` can affect object-derived state.
- Added optimistic ObjectList patching for `useUpdateObject()` and kept object enable/disable responsive by fixing row memo comparison coverage.
- Changed the ObjectList count chip display from `(enabled/total)` to `enabled/total`.
- Expanded ObjectRowItem tests to cover the new chip format and rerender on prop changes.

## Impacted Files

- `src/hooks/useObjects.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridImport.ts` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/preview/hooks/usePreviewPanelActions.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/object-list/ObjectRowItem.test.tsx` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/object-list/useObjHandlersCrud.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/objHandlersHelpers.ts` (modified)
- `src/features/scanner/hooks/useDedup.ts` (modified)

## Goal

ObjectList now repaints promptly for direct object edits, object enable/disable actions, mod-content mutations, manual refresh, and watcher-driven disk changes.

## Impact

- Object row counts and disabled visuals now settle via active refetch instead of waiting for a later screen refresh.
- Object edit flows update the visible row immediately, then reconcile with backend truth.
- No API or schema changes were introduced.

## Notes

- Count math remains backend-authoritative; the frontend only optimistically patches row fields it directly owns.
