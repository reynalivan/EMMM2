# Onboarding snapshot indexing

## Context

Onboarding measured filesystem work and then performed another full disk traversal to reconcile it.

## Changes

- Added a short-lived onboarding snapshot session: one metadata walk now supplies the work plan, terminal-mod sizes, and reusable discovery.
- Added scoped snapshot refresh, raw watcher invalidation, a scheduled 15-minute expiry, and full-reconcile fallbacks for uncertain watcher state.
- Added onboarding commands, permission entries, generated bindings, snapshot/work-plan events, and UI cleanup/update handling.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs` (added)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/disk_snapshot.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/{mod.rs,types.rs,reconcile.rs,reconcile_tests.rs}` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/{entry.rs,request.rs,run.rs}` (modified)
- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs`, `src-tauri/src/lib.rs`, `src-tauri/permissions/app-commands.toml` (modified)
- `src/pages/onboarding/WelcomeScreen.tsx`, `src/pages/onboarding/WelcomeScreen.test.tsx` (modified)
- `src/shared/api/tauri/{bindings.ts,bindings.gen.ts}` (modified)

## Goal

Onboarding reuses a verified filesystem snapshot while preserving correction of external changes.

## Impact

Clean onboarding avoids the duplicate metadata traversal. Watcher uncertainty deliberately falls back to a full reconcile; no database migration or dependency was added.

## Notes

Frontend typecheck and focused UI tests passed. Native full-suite compilation is currently blocked by unrelated concurrent hotkey source errors.
