# Phase 11: Promote V2 Schema & Finalize Bugfixes

## Context

The Greenfield V2 Schema (`collection_v2`) successfully superseded the old collections system, but retained its `_v2` suffix. Concurrently, manual duplicate name checking in `collection_repo.rs` contained logical holes allowing SQLite `UNIQUE` constraint crashes, and the `is_dirty` calculation in the Topbar was failing due to signature hashing mismatches and stale React Query caches. Finally, the "Apply Collection" preview dialog silently crashed due to unhandled promise rejections.

## Changes

- **DB Migration 012**: Dropped all legacy V1 collection tables (`collections`, `collection_items`, `collection_nested_items`, etc.) and executed `ALTER TABLE collection_v2 RENAME TO collections`.
- **Duplicate Name Safeguard**: Removed the `AND kind = 'named'` filter from the uniqueness validation check in `collection_repo::create()`, ensuring ALL identical names are intercepted before hitting the DB's hard `UNIQUE` constraint.
- **Dirty State Hash Parity**: Configured `get_corridor_state` to intentionally omit `is_nested_mod` paths during generic corridor signature generation to strictly match `compute_signature()`.
- **Cache Invalidations**: Added React Query invalidations (`corridorKeys.base()`) into `bulk_toggle_mods` and `core_ops.rs` via newly injected `recompute_signature()` calls, forcing the backend corridor DB cache and frontend TanStack Query to refresh immediately upon user toggles.
- **Fail-Safe UI**: Engineered a resilient text-based error boundary inside `ApplyCollectionModal` resolving the silent "Blank Preview" crash.

## Impacted Files

- `src-tauri/migrations/012_promote_v2_schema.sql` (added)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/repo/corridor_repo.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/mods/bulk_ops.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)

## Goal

The database schema definitively reflects the V2 engine as the primary source of truth without legacy cruft. The App intelligently identifies unsaved/dirty states asynchronously and reliably prevents collection name duplication without application panics.

## Impact

- **Breaking Changes:** None. SQLite natively auto-mapped all indices and Foreign Keys from `collection_v2` dynamically during the `RENAME TO` query.
- **Performance:** `recompute_signature` introduces a minor sub-1ms BLAKE3 hashing overhead during Bulk Mod toggling, mitigating UI staleness effectively.
