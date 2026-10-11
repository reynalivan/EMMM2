# Improve configured startup performance

## Context

Approved startup plan: retain valid watcher/recovery ownership, reduce Windows
registration cost, hydrate the shell early, and remove redundant projection work.

## Changes

- Reuse the current same-game watcher or join its physically bound recovery owner.
- Separate receiver lifetime from activation publication; reject ended coverage,
  stale owners, native root/runtime-parent changes and incomplete caches.
- Build owned watcher candidates in the blocking pool; Windows caches directory
  IDs with parallel traversal and efficient subtree pruning.
- Keep active coverage when repairing an inactive game; check the raw onboarding
  lease after snapshot application before accepting readiness.
- Hydrate revisioned settings/provisional selection early; cancel stale startup
  queries before refresh. Explicit switches retain their gate.
- Match object-key namespaces and reuse the post-object index when mod staging
  did not change it. No schema, dependency or marker-version change.

## Impacted Files

- `src-tauri/AGENT.md`
- `src-tauri/src/modules/reconciliation/`: `adapters/tauri/disk_reconcile_cmds.rs`,
  `application/disk_reconcile/orchestrator/{entry,state}.rs`,
  `application/disk_reconcile/projection_writer/{write,tests}.rs`
- `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`
- `src-tauri/src/modules/system/`: `application/app/bootstrap.rs`,
  `adapters/sqlite/utils/unicode_keys.rs`
- `src-tauri/src/modules/workspace/`: `api.rs`, `adapters/mod.rs`,
  `adapters/notify/{mod,identity_cache}.rs`,
  `adapters/tauri/{mod,watcher_activation}.rs`,
  `application/scanner/watcher/{mod,suppressor,lifecycle,registration,activation_snapshot}.rs`,
  `application/scanner/tests/{watcher_tests,native_file_moves}.rs`
- `src-tauri/tests/startup_watcher_audit.rs`
- `src/app/entrypoint/{App.tsx,App.test.tsx}`
- `src/app/store/{appStore/gameSlice.ts,useAppStore.test.ts}`
- `src/shared/lib/{queryRefresh.ts,queryRefresh.test.ts}`
- `src/pages/dashboard/hooks/{useDashboardStats.ts,useDashboardStats.test.ts}`
- `tests/e2e/specs/startup-reopen.e2e.ts`
- `docs/plans/startup-performance/{implementation_plan,validation}.md`
- This history file

## Goal and Impact

Reduce repeated startup work while preserving filesystem truth and mutation
guards. Paired registration measured 40.87 seconds for the SDK full cache and
0.52 seconds for the production directory cache on the same library. These are
stage timings, not installed-app startup results. Three native process restarts
each performed one startup baseline and no duplicate game-switch scan. Full
offline validation remains; the installed executable was not replaced.

## Validation

Frontend: 1,264 tests passed/one skipped, production build and typechecks passed.
Native: focused watcher suite 117 passed in three consecutive parallel runs;
the rebuilt application passed the E2E test with three actual process restarts.
The latest whole-library run passed 1,504 tests and failed four: three owned
timing assumptions were corrected and verified in the repeated focused runs;
the remaining Catalog command-permission failure belongs to concurrent work.
Review findings were fixed and reviewed again. Compiler results and measurement
limits are recorded in `validation.md`; final whole-target Clippy passed with
`-D warnings`, and documentation formatting and diff checks passed.
