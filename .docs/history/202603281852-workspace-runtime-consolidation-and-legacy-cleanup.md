# Workspace Runtime Consolidation And Legacy Cleanup

## Context

`ObjectList`, `FolderGrid`, and `PreviewPanel` were still assembling core runtime state from separate queries and fallback resolvers. That left the `mods` workspace with duplicate fetches, frontend-only semantic guessing, and legacy helpers that no longer matched the new runtime flow.

## Changes

- Added backend `WorkspaceViewModel` read-model and `get_workspace_view_model` command so the `mods` workspace can read object rows, explorer state, preview selection, and runtime metadata from one payload.
- Moved main `mods` consumers to that payload: ObjectList reads `workspace.objects`, FolderGrid reads `workspace.explorer`, and PreviewPanel reads `workspace.preview`.
- Extended optimistic cache helpers so object and folder updates patch the workspace query alongside legacy object/folder queries during the migration window.
- Replaced the old preview target fallback resolver with backend-selected preview nodes; the obsolete preview resolver module was removed.
- Removed a duplicate object fetch from `MoveToObjectDialog`; the dialog now uses workspace-provided objects when available and only falls back to its own lookup when needed.
- Tightened runtime refresh in primary grid flows to use the shared runtime refresh bus instead of ad-hoc folder/object invalidation.
- Added backend workspace service tests for flat-root preview resolution and nested current-path projection.
- Removed dead preview helper wrappers that no longer had real consumers.

## Impacted Files

- `src-tauri/src/domain/workspace.rs` (added)
- `src-tauri/src/commands/app/workspace_cmds.rs` (added)
- `src-tauri/src/services/workspace_service.rs` (added)
- `src-tauri/src/domain/mod.rs` (modified)
- `src-tauri/src/commands/app/mod.rs` (modified)
- `src-tauri/src/services/mod.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/explorer/listing.rs` (modified)
- `src-tauri/src/services/explorer/types.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src/types/workspace.ts` (added)
- `src/lib/bindings.ts` (modified)
- `src/features/workspace-runtime/useWorkspaceViewModel.ts` (added)
- `src/stores/useAppStore.ts` (modified)
- `src/features/runtime-sync/queryRefresh.ts` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/useObjectListLogic.test.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridImport.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/folder-grid/MoveToObjectDialog.test.tsx` (modified)
- `src/features/folder-grid/FolderGridModals.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.test.ts` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/preview/hooks/usePreviewData.test.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.test.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/preview/components/PreviewPanelModals.tsx` (modified)
- `src/features/preview/previewTargetResolver.ts` (removed)
- `src/features/preview/previewTargetResolver.test.ts` (removed)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)

## Goal

The `mods` workspace now has one runtime source of truth for its main panels, with less duplicate fetching, less frontend semantic inference, and cleaner migration away from legacy preview/runtime glue code.

## Impact

- Main panel state stays aligned more reliably because object rows, explorer children, and preview selection come from the same backend payload.
- Move-to-object flows are cheaper in the active workspace because they reuse existing object rows instead of always issuing another object query.
- Refresh behavior in core workspace surfaces is more predictable because the runtime bus now drives the main re-fetch path.
- Breaking change risk is limited to the active `mods` workspace surface; legacy queries remain available behind the scenes during the transition.

## Notes

- Rust test binaries still fail to execute in this Windows environment with `STATUS_ENTRYPOINT_NOT_FOUND`, so backend verification for the new workspace service was limited to `cargo test --no-run`.
