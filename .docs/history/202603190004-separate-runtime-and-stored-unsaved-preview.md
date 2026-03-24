## Separate runtime current state from stored unsaved snapshot

### Context

- Collections UI still conflated the live current runtime state with the stored `is_last_unsaved` snapshot.
- Safe-mode switch preview still hid object-state differences.
- Historical collections with empty `collection_roots` and `collection_signatures` were not materialized deterministically.

### Changes

- Split frontend workspace selection into typed corridor-scoped sources: live `current_runtime` vs `stored_collection`.
- Kept stored unsaved snapshots visible even when the strict active state is a named collection.
- Routed workspace preview by source type instead of inferring from `is_last_unsaved`.
- Unified save/apply success flows to reselect the resulting named collection through the shared corridor selection state.
- Extended safe-mode switch preview payloads to include object states on both leaving and target sides.
- Switched switch preview rendering to the same object-aware grouped renderer used by collections/apply.
- Replaced runtime materialization count shortcuts with per-collection completeness checks and fixed historical backfill coverage.

### Impacted Files

- `src/lib/corridorSelection.ts` (modified)
- `src/features/collections/utils/workspaceSelection.ts` (added)
- `src/stores/useAppStore.ts` (modified)
- `src/stores/useAppStore.test.ts` (modified)
- `src/features/collections/CollectionsPage.tsx` (modified)
- `src/features/collections/CollectionsPage.test.tsx` (modified)
- `src/features/collections/components/CollectionWorkspace.tsx` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.test.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.test.tsx` (modified)
- `src/types/collection.ts` (modified)
- `src-tauri/src/services/corridor_types.rs` (modified)
- `src-tauri/src/services/corridor_runtime.rs` (modified)
- `src-tauri/src/services/collections/runtime_snapshot.rs` (modified)
- `src-tauri/src/database/game_repo.rs` (modified)
- `src-tauri/src/database/collection_repo.rs` (modified)
- `src-tauri/tests/collections_service.rs` (modified)

### Goal

- The app now distinguishes live current runtime state from stored unsaved snapshots, keeps both browsable, and carries object-state semantics through collection/apply/switch previews.

### Impact

- Collections page selection is now corridor-scoped and typed instead of a plain collection id.
- Safe-mode preview shows object-state changes, not only mods.
- Historical collections without runtime materialization are backfilled more reliably before strict matching.
- No schema change was introduced in this pass.

### Notes

- Strict topbar state still depends on backend runtime matching and remains intentionally separate from workspace selection.
