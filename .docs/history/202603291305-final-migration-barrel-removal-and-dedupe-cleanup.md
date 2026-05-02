## Title

Final migration cleanup for barrel removal, semantic dedupe, and runtime audit hardening

## Context

The workspace/runtime migration was functionally stable, but a few convenience barrels, duplicated semantic helpers, and stale test/module paths still kept the repo from being fully clean.

## Changes

- Removed the last production convenience barrels for folder/object and collections hooks.
- Migrated remaining production consumers to direct owner-module imports.
- Centralized collection preview semantic formatting into one adapter instead of local formatting inside the tree component.
- Unified `MatchedDbEntry` usage to the generated binding type.
- Moved object-list switch decision logic and object context-menu enabled-state derivation to shared switch policy helpers.
- Removed one leftover non-runtime convenience re-export from the welcome demo module.
- Updated audit/tests so deleted barrels and removed compatibility paths are no longer referenced.

## Impacted Files

- Public barrels removed:
  - `src/hooks/useFolders.ts` (removed)
  - `src/hooks/useObjects.ts` (removed)
  - `src/features/collections/hooks/index.ts` (removed)
  - `src/features/collections/index.ts` (removed)
- Production consumers updated:
  - `src/features/launch-bar/LaunchBar.tsx` (modified)
  - `src/features/file-management/TrashManagerModal.tsx` (modified)
  - `src/features/scanner/ScannerFeature.tsx` (modified)
  - `src/components/layout/top-bar/GameSelector.tsx` (modified)
  - `src/components/layout/top-bar/ContextControls.tsx` (modified)
  - `src/components/layout/top-bar/GlobalActions.tsx` (modified)
  - `src/features/settings/tabs/PrivacyTab.tsx` (modified)
  - `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- Semantic/policy cleanup:
  - `src/features/collections/collectionPreviewSemantics.ts` (added)
  - `src/features/collections/components/CollectionTreeView.tsx` (modified)
  - `src/features/object-list/SyncConfirmModal.tsx` (modified)
  - `src/features/object-list/ObjectListModals.tsx` (modified)
  - `src/features/object-list/ObjectListContent.tsx` (modified)
  - `src/features/object-list/ObjectContextMenuTarget.ts` (modified)
  - `src/features/workspace-runtime/actions/workspaceSwitchPolicy.ts` (modified)
- Welcome/demo cleanup:
  - `src/features/welcome/demoTypes.ts` (modified)
  - `src/features/welcome/AnimatedLogo.tsx` (modified)
  - `src/features/welcome/SmartDemoStrip.tsx` (modified)
  - `src/features/welcome/scenes/DemoAutoOrganize.tsx` (modified)
  - `src/features/welcome/scenes/DemoKeybindSpotlight.tsx` (modified)
  - `src/features/welcome/scenes/DemoTogglePreset.tsx` (modified)
- Test and audit updates:
  - `src/features/launch-bar/LaunchBar.test.tsx` (modified)
  - `src/features/file-management/TrashManagerModal.test.tsx` (modified)
  - `src/features/object-list/CreateObjectModal.test.tsx` (modified)
  - `src/features/object-list/EditObjectModal.test.tsx` (modified)
  - `src/features/object-list/useObjectListHandlers.test.ts` (modified)
  - `src/features/object-list/useObjectListLogic.test.ts` (modified)
  - `src/features/folder-grid/FolderGrid.test.tsx` (modified)
  - `src/features/folder-grid/FolderCardContextMenu.test.tsx` (modified)
  - `src/features/folder-grid/hooks/useFolderGrid.test.ts` (modified)
  - `src/features/mod-runtime/operations/sharedOperations.test.ts` (modified)
  - `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (modified)
  - `src/features/workspace-runtime/optimistic/descriptorBuilders.test.ts` (modified)
  - `src/hooks/useObjects.test.tsx` (modified)

## Goal

The codebase now uses direct owner-module imports, shared semantic/policy helpers, and centralized runtime audit rules without the leftover convenience barrels from the pre-final architecture.

## Impact

- Main runtime consumers no longer depend on removed barrel APIs.
- Duplicate switch/status derivation is reduced, which lowers drift risk between object list, grid, preview, and collection preview surfaces.
- No breaking DB changes were introduced.
- Test coverage now protects against reintroducing deleted barrels and removed compatibility paths.

## Notes

- Raw query invalidation remains intentionally limited to central infra modules.
- Collection preview still uses a frontend adapter for its own payload shape instead of forcing a backend contract rewrite.
