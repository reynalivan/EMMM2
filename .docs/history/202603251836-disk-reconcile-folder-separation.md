# Disk Reconcile Folder Separation

## Context

Public command names already separated Disk Reconcile from Deep Match Scanner, but backend folder and module names still used mixed legacy paths like `runtime_sync` and `sync_cmds`.

## Changes

- Renamed the backend runtime folder from `services/runtime_sync` to `services/disk_reconcile`.
- Renamed scanner command files from `runtime_sync_cmds.rs` and `sync_cmds.rs` to `disk_reconcile_cmds.rs` and `deepmatch_scanner_cmds.rs`.
- Renamed the scanner command test file to match the Deep Match Scanner command domain.
- Updated all Rust module exports, imports, Tauri registrations, and watcher references to the new folder/module paths.

## Impacted Files

- `src-tauri/src/services/disk_reconcile/mod.rs` (moved)
- `src-tauri/src/services/disk_reconcile/orchestrator.rs` (moved, modified)
- `src-tauri/src/services/disk_reconcile/path_classifier.rs` (moved)
- `src-tauri/src/services/disk_reconcile/reconcile.rs` (moved, modified)
- `src-tauri/src/services/disk_reconcile/types.rs` (moved)
- `src-tauri/src/services/disk_reconcile/watcher_batch.rs` (moved)
- `src-tauri/src/services/mod.rs` (modified)
- `src-tauri/src/commands/scanner/disk_reconcile_cmds.rs` (moved, modified)
- `src-tauri/src/commands/scanner/deepmatch_scanner_cmds.rs` (moved, modified)
- `src-tauri/src/commands/scanner/tests/deepmatch_scanner_cmds_tests.rs` (moved)
- `src-tauri/src/commands/scanner/mod.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/lib.rs` (modified)

## Goal

Disk Reconcile and Deep Match Scanner are now separated not only by public API names, but also by backend folder and module paths.

## Impact

- Breaking change: old Rust module paths `services::runtime_sync`, `commands::scanner::runtime_sync_cmds`, and `commands::scanner::sync_cmds` no longer exist.
- Internal domain boundaries are clearer for future edits and AI-assisted changes.
- No runtime behavior change; this is a structural clarity refactor.

## Notes

- `cargo check`, `pnpm exec tsc --noEmit`, and the targeted Vitest suites passed after the folder/module rename.
