# Fix onboarding watcher handoff

## Context

Onboarding could finish disk projection but repeatedly reject Confirm because watcher continuity was never established.

## Changes

- Pass the activation guard already held by onboarding to the inactive watcher installer. Previously the wrapper tried to acquire the same guard again and always skipped installation.
- Keep the existing snapshot-change and watcher-authority checks; no disk result is trusted solely because indexing completed.

## Impacted Files

- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs` (modified)
- `docs/history/202609290001-fix-onboarding-watcher-handoff.md` (added)

## Goal

Allow a clean first-game index to complete onboarding while preserving watcher-backed readiness.

## Impact

Fixes repeated onboarding failure for the first inactive game. The watcher is installed during the existing guarded handoff; no schema or dependency change.
