# Barrel Cleanup, Refresh Registry, and Final Audit

## Context

Runtime migration was already stable, but internal code still leaked through public barrels, refresh scopes were repeated across features, preview title/draft derivation was duplicated, and several tests/audits still reflected pre-refactor import paths.

## Changes

- Centralized repeated runtime refresh scope arrays into `buildRuntimeMutationDescriptor(...)`.
- Moved internal runtime imports off public barrels and onto owner modules.
- Removed dead switch compatibility hook `useEnableOnlyThis`.
- Consolidated preview title/subtitle and metadata-change derivation into preview runtime hooks.
- Aligned duplicate warning payloads on the shared `DuplicateInfo` type.
- Hardened architecture audit coverage for:
  - no internal runtime imports through `useFolders` / `useObjects`
  - no old switch commands in frontend source
  - no raw `invalidateQueries(...)` outside central infra
  - no literal refresh-event arrays outside descriptor infra
- Repaired stale test mocks/assertions after the owner-module split.

## Impacted Files

- `src/features/workspace-runtime/optimistic/descriptorBuilders.ts` (modified)
- `src/features/workspace-runtime/optimistic/descriptorBuilders.test.ts` (added)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.test.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.test.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/scanner/hooks/useDedup.ts` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/folder-grid/IgnoreManagementModal.tsx` (modified)
- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/BulkTagModal.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridBulk.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridImport.ts` (modified)
- `src/features/object-list/CreateObjectModal.tsx` (modified)
- `src/features/object-list/AutoSetupModal.tsx` (modified)
- `src/features/object-list/EditObjectModal.tsx` (modified)
- `src/features/object-list/EditObjectModal.test.tsx` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/hooks/useEditObjectForm.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/preview/hooks/usePreviewRuntime.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)
- `src/features/preview/hooks/useMetadataDraft.ts` (modified)
- `src/hooks/useFolderCoreMutations.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useObjectMutations.ts` (existing consumer target)
- `src/hooks/useObjectQueries.ts` (existing consumer target)
- `src/hooks/useObjects.ts` (modified)
- `src/hooks/folderCache.ts` (modified)
- `src/types/mod.ts` (modified)
- `src/types/scanner.ts` (shared duplicate payload owner)

## Goal

Internal runtime code now uses narrower owner modules, refresh decisions are centralized, preview derivation has a single source of truth, and the migration is protected by repo-level architecture audits.

## Impact

- Enabled-state and mutation refresh flows are more uniform and easier to audit.
- Public barrels remain available for external/non-runtime callers without leaking orchestration back into runtime internals.
- No breaking DB changes.
- Tests now align with the post-migration module layout.

## Notes

- `useFolders.ts` and `useObjects.ts` are intentionally retained as external-facing convenience barrels.
- Raw invalidation is still allowed only in `queryRefresh.ts` and `applyOptimisticEffects.ts`.
