# Watcher authority for mutation reconciliation

## Context

Rapid toggles, collection apply, and folder-conflict repair could project disk changes without keeping the watcher authority revision aligned. A later mutation could then be rejected as if indexing were incomplete.

## Changes

- Internal preflight and terminal mutation reconciliation now record watcher authority while retaining the game and operation locks. Dirty watcher evidence escalates to a full pass, and a changed watcher during validation gets one full retry. If disk projection applied but watcher proof remains pending, the result carries an explicit warning instead of triggering rollback.
- A completed core index may run repair preflight while watcher authority is dirty; actual disk mutation still requires a ready authority token.
- Preflight checks readiness again after reconciliation; a concurrent watcher event cannot silently authorize a mutation.
- Toggle projection leaves its durable journal pending when watcher authority rejects the projected revision.
- Interactive collection apply, including object-only renames, performs a terminal authority pass under its mutation lease; post-commit failure becomes a sync warning, not a request to repeat disk renames.
- Rename and bulk-delete responses preserve authority warnings; disk reconcile UI reports pending indexing validation without presenting an applied disk change as a failed mutation.
- Added regression coverage for consecutive preflight and dirty watcher catch-up.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/entry.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/tests.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/emit.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/types.rs` (modified)
- `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs` (modified)
- `src-tauri/src/modules/collections/adapters/tauri/tauri.rs` (modified)
- `src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs` (modified)
- `src-tauri/src/modules/library/adapters/tauri/mod_core_cmds.rs` (modified)
- `src-tauri/src/modules/library/adapters/tauri/conflict_cmds.rs` (modified)
- `src-tauri/AGENT.md` (modified)
- `src/shared/api/tauri/bindings.gen.ts` (regenerated)
- `src/features/file-watcher/hooks/useFileWatcher.ts` (modified)
- `src/features/file-watcher/hooks/useFileWatcher.test.ts` (modified)
- `src/shared/i18n/locales/en/common.json` (modified)
- `src/shared/i18n/locales/id/common.json` (modified)
- `src/shared/i18n/locales/zh/common.json` (modified)
- `docs/history/202609280002-watcher-authority-mutation-reconcile.md` (added)

## Goal

Disk-backed mod state remains authoritative while rapid toggles, collection apply, conflict repair, watcher events, and indexing converge without revision drift.

## Impact

- Dirty watcher evidence can require a full disk pass; clean rapid toggles keep their scoped path.
- Collection apply with actual renames now includes a post-commit full pass and may take longer on large libraries.
- No schema changes; the reconcile warning enum gains the additive `AuthorityPending` value.
