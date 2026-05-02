# Startup Disk Reconcile Migration

## Context

Boot still used `startup_sync` and watcher startup still ran legacy object GC before entering Disk Reconcile. That kept the migration half-finished and hid the real source of physical `DISABLED ` renames.

## Changes

- Replaced startup boot sync with direct `Disk Reconcile` runs for every configured game.
- Added `StartupBoot` as an explicit Disk Reconcile reason so boot refresh is traceable and silent in the UI.
- Removed watcher pre-GC so watcher startup is now trigger-only and fully delegates to Disk Reconcile.
- Removed `startup_sync` from the service graph and deleted the legacy file.
- Reworded legacy helper comments on `gc_lost_objects` and `object_sync` so they are clearly test/maintenance-only, not product runtime paths.

## Impacted Files

- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/disk_reconcile/types.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/services/mod.rs` (modified)
- `src-tauri/src/services/startup_sync.rs` (removed)
- `src-tauri/src/services/objects/query.rs` (modified)
- `src-tauri/src/services/scanner/object_sync.rs` (modified)
- `src/lib/bindings.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)

## Goal

Startup and watcher boot now use one runtime truth path: Disk Reconcile.

## Impact

- Product boot no longer depends on `startup_sync`, `object_sync`, or `gc_lost_objects`.
- Watcher startup no longer mutates DB before Disk Reconcile runs.
- Existing physical `DISABLED ` folders on disk are preserved; this change only fixes the boot reconciliation path.
- Boot-triggered toasts stay silent because `StartupBoot` is treated as a non-user-facing refresh reason.

## Notes

- Physical rename flows still live in collection apply and corridor switch pipelines; those are now easier to audit because legacy startup sync is gone.
