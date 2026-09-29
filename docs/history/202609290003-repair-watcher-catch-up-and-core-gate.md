# Repair watcher catch-up and core readiness

## Context

An isolated physical rename could leave watcher authority dirty without a reconcile event. A transient unavailable watcher result could then erase preflight eligibility after onboarding and lock the first mod switch.

## Changes

- Treat an ambiguous one-path rename as requiring a conservative full disk reconcile unless it is suppressed.
- Retry rejected or failed watcher reconciliation from current authority evidence, with immediate catch-up followed by capped backoff while dirty; a new event interrupts the wait.
- Keep the verified core-index result as the repair-preflight entitlement. Mutation still requires the latest applied disk projection and clean watcher authority.
- After a recovered activation, settle onboarding and publish the disk result before Ready without duplicate runtime work; accept that newer disk revision in the frontend even when the activation generation is unchanged.
- Keep onboarding waiting through a recoverable activation error until verified Ready arrives or its bounded timeout expires.
- Add regression tests for rename classification, catch-up scheduling, onboarding-to-preflight readiness, and later unavailable or untrusted watcher results.

## Impacted Files

- `src-tauri/src/modules/workspace/application/scanner/watcher/mod.rs` (modified)
- `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/modules/workspace/application/scanner/tests/watcher_tests.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs` (modified)
- `src/app/store/appStore/gameSlice.ts` (modified)
- `src/app/store/useAppStore.test.ts` (modified)
- `src/app/entrypoint/waitForGameActivationReady.ts` (modified)
- `src/app/entrypoint/waitForGameActivationReady.test.ts` (modified)
- `src-tauri/AGENT.md` (modified)
- `docs/plans/priority-game-indexing/implementation_plan.md` (modified)
- `docs/history/202609290003-repair-watcher-catch-up-and-core-gate.md` (added)

## Goal

Physical renames converge from disk without requiring another filesystem event, and a verified onboarding index cannot be permanently re-locked by a later transient watcher failure.

## Impact

Ambiguous rename events may use a full scan. Repeated unavailable-source scans back off up to 30 seconds and do not hold mutation locks while waiting. No schema or dependency change.
