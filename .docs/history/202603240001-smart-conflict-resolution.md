# Smart Conflict Resolution & Ignore Management

## Context

Overhaul the conflict detection logic to be less intrusive for variant mods and providing a robust "Ignore" workflow for power users.

## Changes

- **Database**: Added `ignored_object_conflicts` table to persist user-ignored conflict combinations.
- **Backend Service**:
  - `conflict/mod.rs`: Implemented variant container detection (implicit swapping without warning).
  - `conflict_repo.rs`: Added methods to list, check, and revoke ignored conflicts with mod name enrichment.
  - `toggle_mod_inner_service`: Integrated duplicate detection return (Structured Error) and implicit variant swap.
- **Frontend UI**:
  - `ObjectConflictModal`: Radio-button resolution and "Ignore Warning" persistence.
  - `IgnoreManagementModal`: List and revoke ignored conflicts.
  - `useAppStore`: Added global conflict resolution state and actions.
  - `useFolderGridActions`: Refactored to adopt backend-driven global conflict flow.

## Impacted Files

- `src-tauri/src/repo/conflict_repo.rs` (modified)
- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)
- `src-tauri/src/domain/mods.rs` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/folder-grid/FolderGridModals.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/folder-grid/ObjectConflictModal.tsx` (added)
- `src/features/folder-grid/IgnoreManagementModal.tsx` (added)
- `src/hooks/useFolders.ts` (modified)

## Goal

Establish a non-blocking conflict system that persists user intent (Ignore) and handles variants automatically.

## Impact

- **User Experience**: Fewer false-positive warnings; intuitive resolution modal.
- **Workflow**: Persistent "Ignore" persists across sessions.
- **Performance**: Corridor-scoped conflict checks minimize DB overhead during rapid toggling.
