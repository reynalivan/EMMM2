# Universal workspace node and switch engine finalization

## Context

Main `mods` runtime still had two architectural gaps: preview node typing was narrower than the rest of the workspace contract, and enabled/disabled switch behavior still leaked through legacy hooks and side paths.

## Changes

- Unified preview node typing to the shared `WorkspaceNode` family and added explicit node guards for frontend consumers.
- Renamed the backend preview payload to use the same node family semantics end-to-end and updated workspace service/tests for the new union shape.
- Moved main switch execution out of `useFolders` into `useWorkspaceSwitchActions`, including mod toggle, object toggle, path rewrite, thumbnail eviction, and refresh scope publishing.
- Removed dead legacy switch code:
  - `useToggleMod` from `useFolders`
  - `toggleObjectRootAndRefresh` from shared operations
- Migrated `FolderListRow` to the shared switch UI components so ObjectList, FolderGrid card, FolderGrid list, and Preview render the same switch contract.
- Routed bulk object enable/disable through the shared switch engine instead of the old direct object toggle helper.
- Routed active-context disable flow through the shared switch engine and updated conflict modal ignore flow to reuse the switch engine for enabling.

## Impacted Files

- `src/types/workspace.ts` (modified)
- `src-tauri/src/domain/workspace.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)
- `src/features/preview/hooks/usePreviewRuntime.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.test.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/folder-grid/ObjectConflictModal.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.test.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.test.ts` (modified)
- `src/hooks/useFolders.ts` (modified)

## Goal

`mods` runtime now uses one node family and one switch engine across ObjectList, FolderGrid, and PreviewPanel, with switch side effects flowing through the central runtime refresh/effect path.

## Impact

- Selection/path rewrite stays aligned when toggle renames a folder path.
- Bulk object enable/disable now shares the same switch semantics and refresh vocabulary as single-item toggles.
- Legacy switch helpers are removed from active source, reducing drift risk.
- Remaining direct switch command outside the engine is limited to duplicate resolution via `enableOnlyThis` in the conflict modal.

## Notes

- Heavy folder/object mutation infra still lives in `useFolders` and `useObjects`, but the main switch path no longer depends on their legacy orchestrator hooks.
