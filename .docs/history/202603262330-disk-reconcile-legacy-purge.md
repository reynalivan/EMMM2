# Disk Reconcile Legacy Purge

## Context

Startup and watcher had already migrated to Disk Reconcile, but orphan legacy code still existed in settings, object query helpers, scanner exports, and tests.

## Changes

- Removed `sync_timestamps` from app settings load/save/default state.
- Removed legacy object GC and object sync runtime helpers from product source.
- Migrated old GC-focused tests to call `reconcile_disk_projection` directly.
- Simplified object command test so it no longer depends on the deleted legacy sync helper.
- Removed the unused `AppHandle` requirement from the core Disk Reconcile projection function.

## Impacted Files

- `src-tauri/src/services/config/models.rs` (modified)
- `src-tauri/src/services/config/mod.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/disk_reconcile/reconcile.rs` (modified)
- `src-tauri/src/services/disk_reconcile/orchestrator.rs` (modified)
- `src-tauri/src/services/objects/query.rs` (modified)
- `src-tauri/src/services/objects/tests/query_tests.rs` (modified)
- `src-tauri/src/commands/objects/tests/object_cmds_tests.rs` (modified)
- `src-tauri/src/services/scanner/mod.rs` (modified)
- `src-tauri/src/services/scanner/object_sync.rs` (removed)

## Goal

Disk Reconcile is now the only runtime reconciliation path in product code, with legacy startup/object sync helpers fully removed.

## Impact

- Product source no longer references `startup_sync`, `object_sync`, `gc_lost_objects`, or `sync_timestamps`.
- Tests now validate runtime cleanup behavior through Disk Reconcile instead of deprecated helpers.
- Existing databases may still contain an old `sync_timestamps` row, but the app no longer reads or writes it.

## Notes

- Physical renames at boot can still happen only through intentional collection/corridor recovery flows, not through startup reconciliation.
