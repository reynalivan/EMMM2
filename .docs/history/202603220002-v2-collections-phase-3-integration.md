# Phase 3: Integration & Migration of V2 Collections

## Context

Phase 3 of the Greenfield System Redesign requires migrating all legacy collection hooks and modals to the new v2 equivalents designed in Phase 2, ensuring that all frontend consumers interface correctly with the optimized backend commands.

## Changes

- Swapped `CollectionsPage` routing in `MainLayout.tsx` to use the new `V2CollectionsPage`.
- Migrated Safe Mode toggle consumers (`PrivacyTab.tsx`, `GlobalActions.tsx`, `ContextControls.tsx`) to use `useV2SafeModeToggle`.
- Removed `ApplyCollectionModal` usage from `ContextControls.tsx` in favor of a fast, direct apply mechanism using `useV2ApplyCollection`.
- Implemented `V2SaveCollectionModal` to replace the highly complex 300+ line legacy `SaveCollectionModal` with a simple ~80 line component that relies on the new `v2_create_collection` backend command.
- Integrated `V2SaveCollectionModal` into both `ContextControls` and `V2CollectionsPage`.

## Impacted Files

- `src/components/layout/MainLayout.tsx` (modified)
- `src/features/settings/tabs/PrivacyTab.tsx` (modified)
- `src/components/layout/top-bar/GlobalActions.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src/features/collections/V2CollectionsPage.tsx` (modified)
- `src/features/collections/components/v2/V2SaveCollectionModal.tsx` (added)
- `.docs/v2-migration-cleanup.md` (modified)

## Goal

The system fully consumes the performance-optimized v2 backend endpoints for corridor states, active collections, and snapshotting. The frontend is significantly leaner.

## Impact

- Instant UI response for Safe Mode toggles and Collection Applies.
- Removed complex frontend state management for modals.
- Clean compilation passing.
- Legacy files categorized as "Ready for Deletion" in the cleanup catalog.

## Notes

- `ApplyCollectionModal` and `SaveCollectionModal` are now orphaned and marked for deletion in Phase 5.
