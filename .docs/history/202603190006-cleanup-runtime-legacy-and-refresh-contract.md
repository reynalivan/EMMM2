# Cleanup runtime legacy and refresh contract

## Context

- Collections/corridor wiring was already moved to runtime snapshots, but several backend helpers, comments, and maintenance paths still assumed legacy `collection_items` ownership.
- Save/apply/switch flows also needed one last refresh-contract pass so undo and game/safe-mode transitions use the same runtime refetch path.

## Changes

- Removed unused legacy collection repo helpers that no longer participate in runtime save/apply paths.
- Stopped stable-id migration from rewriting `collection_items`; historical collections now rely on path-based backfill only.
- Simplified `corridor_state` reads to remembered ids only and removed unused `active_collection_name`.
- Removed orphan `collection_items` cleanup from maintenance and expanded reset cleanup to include runtime materialization tables plus `corridor_state`.
- Made undo refetch the same runtime/list contract used by save/apply.
- Fixed the historical backfill regression fixture so it actually simulates legacy `collection_items` input.

## Impacted Files

- `src-tauri/src/database/collection_repo.rs` (modified)
- `src-tauri/src/database/corridor_state_repo.rs` (modified)
- `src-tauri/src/database/settings_repo.rs` (modified)
- `src-tauri/src/services/app/maintenance_service.rs` (modified)
- `src-tauri/src/services/collections/nested_walker.rs` (modified)
- `src-tauri/src/services/collections/runtime_snapshot.rs` (modified)
- `src-tauri/src/services/privacy/tests/privacy_service_tests.rs` (modified)
- `src-tauri/src/services/scanner/sync/helpers.rs` (modified)
- `src-tauri/tests/collections_service.rs` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)

## Goal

- Runtime snapshot and canonical collection materialization stay the real source of truth.
- Legacy item tables remain backfill-only, not active runtime write/read state.

## Impact

- Save/apply/undo/safe-mode flows now refresh from the same runtime contract more consistently.
- Maintenance no longer spends time cleaning runtime-irrelevant legacy collection rows.
- Reset clears new runtime materialization state and remembered corridor pointers.
- No public FE API was reintroduced; strict active state remains runtime-signature based.

## Notes

- Legacy `collection_items` and `collection_nested_items` are still retained for historical backfill coverage and test fixtures.
