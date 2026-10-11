# Stabilize runtime progress and prepare bounded measurements

## Context

Parallel callbacks could regress counts or update state after listener disposal.
Runtime progress also lacked phase units; further backend changes need measurements.

## Changes

- Ignore disposed listeners and lower counts within the same run, phase and total;
  keep terminal reset and legitimate same-ID retry behavior.
- Show folders for scanning, mods for projection and indeterminate finishing.
- Add opt-in owned-fixture harvest dispatch and onboarding priority benchmarks.
- Prepare source snapshots, paired-result summaries and a private release contract.
- Publish WPR handshakes atomically; prepare an Administrator operator helper.

## Impacted files

- `src/features/file-watcher/utils/reconcileProgress.ts` and `.test.ts`.
- `src/widgets/mod-explorer/components/FolderGridSyncToast.tsx` and `.test.tsx`.
- `src-tauri/src/modules/automation/application/keyviewer/harvester.rs` and new
  `harvest_dispatch_benchmark.rs`.
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs`
  and new `onboarding_performance_tests.rs`.
- `docs/plans/startup-performance/final-performance-audit-20261011.md`; this history.
- Ignored `.audit-output/final-performance-audit-20261011/`: `run-wpr-operator.ps1`,
  `snapshot-inputs.mjs`, `summarize-benchmarks.mjs`, `test-native-release-config.mjs`,
  and `wpr-native/` fixture manifest evidence.
- Ignored `.audit-output/bootstrap-bundle-20261011/`: `wdio.wpr-readonly.conf.ts`
  and `test-wpr-gates.mjs`.

## Verification

Progress regressions: 7 initially failing assertions, then 16 targeted tests PASS.
App typecheck, scoped ESLint/formatting and diff checks passed. Independent review
found no actionable frontend defect; its WPR/cache-assertion findings were fixed.
WPR hook contract and helper syntax PASS. Fixture release contract is deliberately
RED until the coordinated native harness change. Rust benchmarks compile within
the cleanup task's debug test build but have not run in release.

## Goal and impact

Progress is bounded and lifecycle-safe with phase-specific units. No new runtime
cache, scheduler, dependency or throughput gain is claimed. Real GIMI was untouched.
Benchmark/native/WPR remain pending the shared compiler and native handoff after
hotkey/cleanup source stabilizes. The user agreed to operate WPR as Administrator;
the command has not been issued because the capture build is not ready.
