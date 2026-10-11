# Fix onboarding progress and first-index lifetime

## Context

Approved audit found delayed activation failure, expiring live sessions, stale
progress and long roots without visible progress.

## Changes

- Fail terminal activation promptly; guard live session ownership and cancellation.
- Await listeners, scope events to session/game/attempt and preserve monotonic progress.
- Emit throttled intra-root activity; install raw watchers in blocking preparation.
- Retain bounded width 8 after parity benchmarks; use unchanged pure event DTOs.
- Validate real Windows picker/Confirm/background/restart in an isolated QA build.

## Impacted Files

- Backend: `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs`;
  `src-tauri/src/modules/reconciliation/application/disk_reconcile/{onboarding_session.rs,disk_snapshot.rs,census_tests.rs,types.rs,orchestrator/request.rs}`.
- Frontend: `src/app/entrypoint/waitForGameActivationReady{.ts,.test.ts}`;
  `src/pages/onboarding/WelcomeScreen{.tsx,.test.tsx}`;
  `src/pages/onboarding/hooks/useOnboardingDiskProgress{.ts,.test.tsx}`;
  `src/shared/api/tauri/{bindings.ts,events.ts}`.
- Clippy cleanup: `src-tauri/src/modules/catalog/adapters/tauri/object_cmds.rs`;
  `src-tauri/src/modules/library/adapters/tauri/mod_meta_cmds.rs`;
  `src-tauri/src/modules/library/application/mods/core_ops/toggle.rs`.
- Native QA: `tests/e2e/specs/{onboarding.e2e.ts,onboarding-indexing-native.e2e.ts}`;
  `wdio.conf.ts`; `src-tauri/tauri.e2e.conf.json`.
- Guidance/report: `src-tauri/AGENT.md`; `.agent/skills/e2e-automation/SKILL.md`;
  `docs/plans/startup-performance/{implementation_plan.md,onboarding-first-index-20261010.md}`;
  this history entry.

## Goal

Responsive, owned onboarding progress with stable first-index lifecycle.

## Impact

No dependency/schema/IPC payload changes. Production configuration untouched.
1,674 Rust and 58 frontend tests pass; strict Clippy/type/lint/format pass.
Two native cases pass: 6,000 foreground mods, 80 background mods and restart.
Home 7.55 seconds is one synthetic debug observation, not cold/p95 latency.
Private QA profile archived intact; real GIMI excluded from automation.

## Notes

[Detailed measurements and limitations](../plans/startup-performance/onboarding-first-index-20261010.md).
