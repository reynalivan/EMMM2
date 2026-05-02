# ObjectList Runtime Cleanup And Owner Metadata

## Context

ObjectList and FolderGrid were already more responsive, but refresh policy was still scattered, shared runtime helpers lived in the wrong feature, and optimistic object count patches still guessed owner objects from path heuristics.

## Changes

- Centralized core refresh policy into a runtime query coordinator.
- Moved shared runtime mutation helpers out of `object-list` into a neutral `mod-runtime` layer.
- Added folder ownership metadata to explorer payloads so optimistic ObjectList patches use backend-owned object IDs instead of longest-path guessing.
- Replaced legacy FolderGrid `selectedObject` navigation plumbing with `selectedObjectFolderPath`.
- Extracted shared single-item mod actions used by FolderGrid and Preview for toggle/favorite/move flows.
- Removed orphaned ObjectList mod delete dialog/state that no longer had a real UI trigger.

## Impacted Files

- `src/features/runtime-sync/queryRefresh.ts` (added)
- `src/features/mod-runtime/operations/sharedOperations.ts` (added)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (added)
- `src/hooks/useObjects.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridNav.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderGridToolbar.tsx` (modified)
- `src/features/folder-grid/FolderGrid.test.tsx` (modified)
- `src/features/preview/hooks/usePreviewPanelActions.ts` (modified)
- `src/features/object-list/useObjHandlersArchive.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/object-list/useObjHandlersCrud.ts` (modified)
- `src/features/object-list/useObjHandlersDrop.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjectListHandlers.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/ObjectListModals.tsx` (modified)
- `src/features/object-list/ObjectListModals.test.tsx` (modified)
- `src/features/object-list/objHandlersHelpers.ts` (removed)
- `src/types/object.ts` (modified)
- `src-tauri/src/services/explorer/types.rs` (modified)
- `src-tauri/src/services/explorer/listing.rs` (modified)
- `src-tauri/src/commands/folder_grid/mod.rs` (modified)
- `src-tauri/src/commands/folder_grid/listing.rs` (modified)
- `src-tauri/src/services/explorer/tests/helpers_tests.rs` (modified)

## Goal

ObjectList, FolderGrid, Preview, watcher refreshes, and corridor-side mutations now share cleaner runtime plumbing with deterministic ownership-aware optimistic updates.

## Impact

- Object counts can patch immediately without path guessing drift.
- FolderGrid navigation now uses one path-based source of truth.
- Preview and FolderGrid share more runtime action logic, reducing future drift.
- No storage migration was needed; ownership metadata is enriched at runtime.

## Notes

- I kept the existing public hooks and UI behavior where possible, and moved logic under them rather than redesigning the surfaces.
