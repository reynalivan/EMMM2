# Recover leaked reconcile object stages

## Context

Object identity staging could leave `.emmm-reconcile-object-stage-*` in the persisted object name when a stale path key resolved to an otherwise unchanged disk object.

## Changes

- Made staged object rows bypass the unchanged-object fast path so their full disk identity is restored.
- Added self-healing for stage names left by an earlier reconcile.
- Added regression coverage for stale path keys and already-leaked stage names.

## Impacted Files

Backend:

- `src-tauri/src/repo/object_repo/lookup.rs` (modified)
- `src-tauri/src/repo/object_repo/types.rs` (modified)
- `src-tauri/src/services/disk_reconcile/projection_writer/objects.rs` (modified)
- `src-tauri/src/services/disk_reconcile/projection_writer/write.rs` (modified)
- `src-tauri/src/services/disk_reconcile/projection_writer/tests.rs` (modified)
- `src-tauri/src/services/disk_reconcile/rename_confirmation.rs` (modified)
- `src-tauri/src/services/disk_reconcile/source_recovery.rs` (modified)

## Goal

Disk reconcile never exposes internal object-stage names and repairs databases already affected by the leak.

## Impact

- The next reconcile restores affected object names from their disk folders.
- No migration or public API change is required.
- Normal unchanged-object reconciliation keeps its existing fast path.
