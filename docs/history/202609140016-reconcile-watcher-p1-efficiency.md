# Reconcile and watcher P1 efficiency

## Context

Repeated watcher refreshes and broad Health invalidation did unnecessary work while a replaced watcher session could still wait for locks.

## Changes

- Reconcile results now report actual discovery coverage: full, scoped, or none. Thumbnail-only batches remain fast unless repair requires a full scan.
- Repair acknowledgement requires an applied full discovery. Watcher work checks its session at each lock boundary, after reconcile, and before result/progress publication.
- The frontend keeps one active automatic reconcile and only the latest compatible queued context. It ignores stale UI side effects.
- Health report invalidation is now game- and path-scoped. Projection filtering avoids clones and watcher suppression cleanup uses sorted registration IDs.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/{types,reconcile,reconcile_tests}.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/{entry,run,tests}.rs` (modified)
- `src-tauri/src/modules/workspace/application/scanner/watcher/{lifecycle,suppressor}.rs` (modified)
- `src/features/file-watcher/hooks/{useFileWatcher,useFileWatcher.test}.ts` (modified)
- Reconcile result fixtures and `src/shared/api/tauri/bindings.gen.ts` (modified)

## Goal

Avoid obsolete scans, unnecessary Health refetches, projection allocations, and linear suppression cleanup while preserving disk-authoritative reconciliation and mutation safety.

## Notes

Global lint and architecture lint still have unrelated pre-existing errors. Targeted lint, typecheck, Rust tests, registry check, and production frontend build pass.
