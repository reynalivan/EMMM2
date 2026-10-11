# Reduce routine startup disk work

## Context

Approved follow-up priorities 1–2: remove routine storage sizing and defer
recursive import staging cleanup beyond native setup.

## Changes

- Remove recursive size discovery, dedicated sizing queries, projection fields
  and size updates from startup/reconcile/watchers; preserve stored legacy sizes.
- Read dashboard duplicate waste from measured optimizer reports bound to the
  configured root and surviving indexed membership; keep unindexed report members.
- Validate active import references and atomically detach startup garbage into
  owned `.cleanup` quarantine. Native Ready deletes only quarantine in the
  blocking pool; interrupted deletion retries next launch.
- Extend native restart checks with isolated orphan staging cleanup assertions.

## Impacted Files

- Reconciliation: `src-tauri/src/modules/reconciliation/application/disk_reconcile/`
  `{reconcile,disk_snapshot,rename_confirmation,source_recovery}.rs`,
  `projection_writer/{mods,tests}.rs`.
- Library DAL: `src-tauri/src/modules/library/adapters/sqlite/mods/{listing,sync}.rs`.
- Dashboard DAL: `src-tauri/src/modules/dashboard/adapters/sqlite/dashboard/{mod,tests}.rs`.
- Watcher tests: `src-tauri/src/modules/workspace/application/scanner/tests/native_file_moves.rs`.
- Staging: `src-tauri/src/modules/ingestion/application/import_batch/staging.rs`.
- Lifecycle: `src-tauri/src/modules/system/application/app/bootstrap.rs`, `src-tauri/src/lib.rs`.
- E2E: `tests/e2e/specs/startup-reopen.e2e.ts`.
- Documentation: `src-tauri/AGENT.md`,
  `docs/plans/startup-performance/{implementation_plan,validation}.md`, this history.

## Goal and Impact

Routine indexing no longer measures asset storage. Explicit Storage Optimizer
measurement remains available; dashboard waste is the last measured report,
not a live disk total. Unindexed external deletions require another explicit
scan. Startup still reads shallow staging ownership/layout and performs renames,
but recursive payload deletion runs after setup. Native Ready is not a
first-paint guarantee. No dependency, migration or installed-app replacement.

## Validation

Reconcile 211 passed/two ignored; dashboard ten, cleanup six and native moves two
passed. Native E2E passed three restarts with quarantine assertions. The manual
1,000-file fixture measured median setup work 96.7 → 6.6 ms; deletion still runs
in the worker. Build/check, final strict Clippy, scoped formatting and E2E
TypeScript/lint passed. Whole Rust: 1,537 passed/one duplicate-whitelist failure
outside these edits/16 ignored; isolated rerun also failed. Independent review's
deleted-member finding was corrected. Measurement limits are in `validation.md`.
