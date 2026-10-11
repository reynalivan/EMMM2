# Simplify startup policies and scope thumbnail invalidation

## Context

User approved the architecture cleanup and investigation of unrelated-folder
thumbnail cancellation. Changes remain local; GIMI was not accessed or modified.

## Changes

- Workspace's normalizer/API now owns one effective-enabled path predicate used
  by Library, Dashboard, Collections and runtime publication. Semantics and DB
  status remain unchanged; domain tests own the separator/legacy-prefix matrix.
- Rename projection metrics to `initial_index` and `metadata_within_mods` in
  production and manual profiling. Metadata is explicitly a subset of mod time.
- Name the existing entry point `initialize_bootstrap_guards`; native identity
  and read-only checks retain their pre-configuration ordering.
- Reproduce unrelated invalidations exhausting three thumbnail retries, then
  scope cancellation to live matching/overlapping folder attempts. Full reset
  retains its epoch. RAII cleans tickets; the existing semaphore permit stays
  inside the blocking closure even when the async waiter is cancelled.
- A second RED regression found stale publication after source removal through
  a junction alias. Canonicalize the nearest existing ancestor and rejoin missing
  components so invalidation still reaches the in-flight root.
- Remove one redundant `Option.into_iter()` in recovery to satisfy strict Clippy;
  root admission and ownership checks retain their behavior.

## Impacted Files

Paths under `src-tauri/src/` (modified, including previously untracked files):

- `lib.rs`, `modules/mutation/recovery.rs`.
- `modules/workspace/api.rs`, `modules/workspace/domain/normalizer.rs`,
  `modules/workspace/domain/tests/normalizer_tests.rs`.
- `modules/library/api.rs`, `modules/library/adapters/sqlite/mods/listing.rs`,
  `modules/library/adapters/sqlite/mods/tests/listing_tests.rs`.
- `modules/dashboard/adapters/sqlite/dashboard/mod.rs`.
- `modules/collections/application/collection/live_state.rs`.
- `modules/system/application/app/post_apply.rs`,
  `modules/system/application/app/readonly_audit.rs`,
  `modules/system/application/app/tests/dashboard_benchmark.rs`.
- `modules/reconciliation/application/disk_reconcile/reconcile.rs`,
  `modules/reconciliation/application/disk_reconcile/projection_writer/write.rs`,
  `modules/reconciliation/application/disk_reconcile/projection_writer/profile_tests.rs`.
- `platform/images/thumbnail_cache.rs`,
  `platform/images/tests/thumbnail_cache_tests.rs`.

Documentation: `src-tauri/AGENT.md`,
`docs/plans/startup-performance/{implementation_plan,final-stabilization-20261010}.md`,
this history (added). Local build metadata:
`.audit-output/startup-cleanup-20261010/native-final-build.json` (added).

## Validation

- Full Rust library suite: **1,635 passed / zero failed / 21 ignored**, 79.70 s,
  including 16 thumbnail tests and two owned Windows junction regressions.
- Strict all-target Clippy, full Rust formatting, whitespace and E2E TypeScript
  checks pass. Initial checks encountered concurrent incomplete mutation APIs
  and formatting; final gates ran after those sources stabilized.
- Fresh TypeScript/Vite assets and private native debug build pass. Native WDIO:
  **5 cases / 2 specs pass**, sessions `77fd821a3f16a6394c36b020daba136a` and
  `efe50860b7d171c6be9458c96491dde0`. Three 301-mod process restarts retain one
  watcher/baseline each. Four preview cases cover fullscreen, hide/restore,
  persisted clear/restart and fallback texture toggles; image bytes preserved.
- Owned process trees stopped before all three fixture removals. No standalone
  browser, real-library benchmark, production config change or installation.
- Focused read-only review found no remaining actionable scoped finding.

## Goal and Impact

One domain policy reduces drift. Sibling thumbnail invalidations no longer waste
retries; real source/visibility changes still reject stale publication. No new
dependency, daemon, pool, public IPC, persistent generation map or schema change.
Cold/invalidation paths perform canonical metadata lookup; full resets still
cancel all in-flight work. Async cancellation admission is source-reviewed,
without a dedicated cancellation stress test. No whole-startup speed claim.

Validated snapshot: `src-tauri/target/debug/emmm-e2e.exe`, 91,507,200 bytes,
SHA256 `cb4dd7a57d2828d7b6b76932be5eae4716e38bbafbfe0dad2b42ba332c8c5f7f`.
The installed application was not replaced.
