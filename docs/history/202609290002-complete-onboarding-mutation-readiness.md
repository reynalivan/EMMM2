# Complete onboarding mutation readiness

## Context

Onboarding could finish a verified disk reconcile and watcher handoff while the per-game initial recovery gate remained unstarted. Activation then repeated indexing, and a mod switch could be rejected before its disk rename with “Game indexing is not ready.”

## Changes

- After the onboarding watcher handoff, complete the initial recovery gate only when the applied result still matches a clean, trusted watcher authority.
- Keep stale, dirty, or untrusted results blocked. A queued initial recovery checks its generation after taking the game lock and exits before scanning or writing if onboarding has superseded it.
- If takeover happens during a running scan, ignore its superseded completion/event and join the current recovery gate. Register recovery waiters before checking the gate to avoid a missed completion notification.
- Add regression tests for clean, pending, stale, and dirty handoff states, including an outdated scan request that must not record a result.

## Impacted Files

- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/emit.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/entry.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/request.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/tests.rs` (modified)
- `docs/history/202609290002-complete-onboarding-mutation-readiness.md` (added)

## Goal

The first game becomes eligible for disk-first mod switching immediately after verified onboarding, without a redundant activation scan.

## Impact

Activation can reuse the verified onboarding result. Filesystem changes still invalidate watcher authority and require reconciliation before mutation.
