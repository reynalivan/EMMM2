# Stabilize startup validation and retain measured sorting gains

## Context

User approved source/Clippy stabilization, conditional census sort optimization,
projection profiling and final native Windows validation. Real GIMI stays read-only.

## Changes

- Resolve three strict boolean lint findings without removing ownership checks.
- Reuse stable temporary cached sort keys across census collectors: complete
  census median 1,349 → 1,210 ms, all fields/ties preserved.
- Add projection phase timings and an isolated clone profiler. Metadata dominates;
  suppressing 592 unchanged object UPDATEs did not improve total timing, so no
  production SQL guard or read-model skip was added.
- Make native hook failures terminate after owned cleanup, and validate the
  expected private identifier before bootstrap. Controlled driver regression:
  rejected preparation previously sent four session requests; now sends zero.
- Validate canonical profiler isolation, ancestors/junctions and SQLite sidecars.
- Repair invalid dedup test INIs and assert discovered candidates. Isolate
  thumbnail test caches while preserving the production singleton and guards.
- Separate the 1,000-rename lineage stress window from expiry tests; keep the
  production ten-second TTL and repair behavior unchanged under parallel load.

## Impacted files

- `src-tauri/src/modules/mutation/recovery.rs`;
  `src-tauri/src/modules/library/application/mods/trash/service.rs`.
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/`:
  `disk_snapshot.rs`, `census_tests.rs`, `reconcile.rs`,
  `projection_writer/{mod,state,write,mods}.rs`, `projection_writer/profile_tests.rs` (new).
- `src-tauri/src/modules/system/application/app/readonly_audit.rs`.
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_scanner_tests.rs`.
- `src-tauri/src/modules/workspace/application/scanner/watcher/suppressor.rs` (test only).
- `src-tauri/src/platform/images/thumbnail_cache.rs`;
  `src-tauri/src/platform/images/tests/thumbnail_cache_tests.rs`.
- `wdio.conf.ts`, `wdio.readonly-audit.conf.ts`, `tests/e2e/support/driverLifecycle.ts`;
  `scripts/test-native-prepare-interlock.mjs` (new).
- `src-tauri/AGENT.md`, `.agent/skills/e2e-automation/SKILL.md`,
  `docs/plans/startup-performance/final-stabilization-20261010.md` (new),
  `docs/plans/startup-performance/validation.md`, this history.

## Goal and impact

Reduce measured census work without new services, dependencies, persistent cache
or worker pools. Failed validation cannot proceed into a driver session or wrong
app profile. Real metadata inventories match; all DB writes use the managed clone.

## Validation

Cached-sort parity/benchmark, profiler isolation, native identifier and driver
guards pass. Thumbnail tests: eight pass; whitelist regression passes. Strict
all-target Clippy, scoped formatting and E2E types/lint pass. Final parallel Rust
suite: 1,614 passed / zero failed / 21 ignored. Fresh native Windows E2E: one test
passes three 301-mod process restarts; one watcher/baseline each and owned cleanup
complete. Native identity/hash and measurement limits are in the plan.
