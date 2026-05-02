# Phase 4 dead code and legacy runtime purge

## Context

Sesudah runtime `mods` stabil, repo masih menyisakan dead collection utilities, dead runtime state, compatibility refresh shells, dan wrapper tipis yang tidak lagi memberi nilai.

## Changes

- Removed dead collection preview code that no longer had production consumers: `buildModTree`, `groupMods`, and `ModGroupList`.
- Removed dead runtime machine state for pending selection rewrite and drag/drop, plus related events and bridge/store plumbing.
- Migrated collection create/update/delete refreshes to descriptor-driven runtime refresh and removed unused public refresh helpers from `queryRefresh`.
- Inlined the thin `useFolderGridActions` wrapper into `useFolderGrid` and deleted the wrapper file.
- Renamed stale-path repair helper in `useFolders` to an explicit disk repair recovery path.
- Tightened architecture audit coverage to lock out removed legacy store/runtime state.

## Impacted Files

- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCollections.test.ts` (modified)
- `src/features/collections/utils/buildModTree.ts` (removed)
- `src/features/collections/utils/buildModTree.test.ts` (removed)
- `src/features/collections/utils/groupMods.ts` (removed)
- `src/features/collections/components/ModGroupList.tsx` (removed)
- `src/features/runtime-sync/queryRefresh.ts` (modified)
- `src/features/runtime-sync/queryRefresh.test.ts` (modified)
- `src/features/workspace-runtime/state/workspaceState.ts` (modified)
- `src/features/workspace-runtime/state/workspaceEvents.ts` (modified)
- `src/features/workspace-runtime/state/workspaceReducer.ts` (modified)
- `src/features/workspace-runtime/state/workspaceReducer.test.ts` (modified)
- `src/features/workspace-runtime/state/workspaceSelectors.ts` (modified)
- `src/features/workspace-runtime/state/workspaceStoreBridge.ts` (modified)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.test.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.test.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.test.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (removed)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFileDrop.test.tsx` (modified)
- `src/stores/useAppStore.ts` (modified)

## Goal

Reduce runtime and collection surface area to code that is actually used, and keep refresh/state architecture aligned with the descriptor-driven runtime model.

## Impact

- Main `mods` runtime now has less dead state and one less folder-grid wrapper layer.
- Collections mutation refresh behavior now follows the same descriptor-driven path as the rest of runtime code.
- No expected runtime breaking change for active `mods` flows; removed files had no production consumers.
