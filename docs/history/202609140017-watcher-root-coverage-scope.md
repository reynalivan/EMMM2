# Watcher root-coverage scope

## Context

Watcher batches with 128 file events in one object root previously forced a full reconcile solely from event count.

## Changes

- Replaced the event-count trigger with a normalized, deduplicated top-level root count.
- Kept the threshold at 128 distinct roots.
- Force a full reconcile when watcher evidence is lost, empty, outside the mods root, hidden/ambiguous, contains parent traversal, or renames across roots or with a missing side.
- Passed the watched mods root into watcher-batch request construction.
- Added regression coverage for concentrated batches, wide/ambiguous coverage, watcher errors, and cross-root identity conflicts.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/request.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/entry.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/tests.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/reconcile_tests.rs` (modified)
- `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs` (modified)

## Goal

Large bursts confined to one root use scoped discovery, while uncertain watcher input and cross-root conflicts retain full-reconcile safety.

## Impact

No public API, dependency, schema, or filesystem-authority change. Full scans remain mandatory for overflow, rescan/error, ambiguous paths, repair, and detected cross-root conflicts.
