# Fix matching, mutation and reconciliation

## Context

Fix five approved MMR findings, preserving the dirty shared checkout.

## Changes

- Proven replacement mods no longer inherit old manual safety; owned renames
  and compatible legacy identity gaps retain the user's choice.
- Attach learned aliases before MasterDB indexing; include skin aliases in
  candidate tokens. Validate hash bytes before slicing to prevent UTF-8 panic.
- Registration is RAII-owned before Applying persistence, preventing stale task IDs.
- Journal optional source-root identity; import capture/revalidation and recovery
  are limited to proven configured staging/inbox roots. Pin publication/rollback
  ancestry, reject rebinding and preserve no-overwrite behavior. Overlapping
  roots select the recorded identity, not the first containing path.

## Impacted Files

Under `src-tauri/src/` (modified):

- `modules/matching/application/deep_matcher/analysis/indexes.rs`
- `modules/matching/application/deep_matcher/state/master_db.rs`
- `modules/matching/application/deep_matcher/tests/{analysis/indexes_tests.rs,pipeline/quick_pipeline_tests.rs}`
- `modules/workspace/application/scanner/{master_db/cache.rs,tests/master_db_cache_tests.rs}`
- `modules/mutation/{coordinator.rs,journal.rs,recovery.rs,tests.rs}`
- `modules/mutation/application/workspace_mutation/import_commit.rs`
- `modules/reconciliation/application/disk_reconcile/projection_writer/{mods.rs,tests.rs}`
- `modules/reconciliation/application/disk_reconcile/reconcile_tests.rs`
- `modules/reconciliation/adapters/tauri/toggle_projection.rs`
- `modules/system/application/app/bootstrap.rs`

Other files: `src-tauri/AGENT.md` (modified); `.bug-hunter/mmr-fix/` plans,
scopes, exact baseline/hash copies and validation/final report (added); this history (added).

## Goal and Impact

Prevent audited wrong matching/safety and recovery/bookkeeping failures.
No IPC, dependency, SQL migration or journal-version change. Manual repair remains
for legacy outside-root missing-proof and unowned copied targets. Do not downgrade
unfinished new-proof journals to an incompatible reader. No speedup claim.

## Validation

On the previously verified snapshot, 403 Rust tests, 44 frontend tests and
2 fresh-build native Windows cases passed;
7 Rust tests remain intentionally ignored. Strict Clippy, scoped formatting/
whitespace and production frontend/native debug builds passed. Independent
review corrections are closed.

Latest-workspace revalidation is blocked by concurrent out-of-scope retry/
file-lock changes. Native rebuild stopped at TypeScript binding/signature
errors; latest strict Clippy stopped at unresolved `WatcherState` references
in catalog mutation code. Concurrent edits to the shared import implementation
were preserved. The five patches are implemented, but the earlier passing
snapshot does not certify the current checkout. No unrelated fix was applied.

No full repository audit or actual interrupted native import/restart certification;
Linux and external game/OS integration remain unverified. Details:
`.bug-hunter/mmr-fix/validation.md`.
