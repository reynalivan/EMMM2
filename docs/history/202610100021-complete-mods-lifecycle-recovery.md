# Complete Windows Mods lifecycle fixes

## Context

User requested completing Bug Hunter fixes against current shared code and
approved an additive SQLite migration. Work stayed uncommitted on `main`;
concurrent edits were preserved.

## Changes

- Empty-grid navigation rejects out-of-range item access.
- Organizer binds original destination ancestry, creates missing components
  through owned parent handles and routes `only-enable` siblings to their own parent.
- Collection captures original root before waits; retries/rollback retain
  ownership. Trusted projection pins all rebased participants, including no-ops.
- Finalization writes operation/game/root receipt in its SQL transaction.
  Recovery preserves committed work, rejects ambiguity and cleans only terminal proof.
- Windows toggle publishes the verified source handle; deterministic retry
  fixtures replace timer-based race assumptions.

## Impacted Files

```text
src/widgets/mod-explorer/hooks/useFolderNavigation.{ts,test.ts}
src-tauri/src/modules/library/application/mods/{organizer_move.rs,organizer_targets.rs,tests/organizer_move_tests.rs,core_ops/toggle.rs}
src-tauri/src/modules/collections/application/apply/{apply_pipeline.rs,apply_pipeline_tests.rs,steps/batch_rename.rs,steps/rollback.rs}
src-tauri/src/modules/mutation/{mod.rs,api.rs,coordinator.rs,journal.rs,recovery.rs,tests.rs,application/workspace_mutation/engine.rs,adapters/mod.rs,adapters/sqlite/mod.rs,adapters/sqlite/projection_receipts.rs}
src-tauri/src/modules/system/application/app/bootstrap.rs
src-tauri/src/platform/fs/file_utils.rs
src-tauri/migrations/20261010000000_mutation_projection_receipts.sql
src-tauri/AGENT.md
tests/e2e/specs/mods-lifecycle-hardening.e2e.ts
docs/plans/mods-manager-hardening/{lifecycle_implementation_plan.md,lifecycle-validation.md}
.bug-hunter/mods-lifecycle-fix/ (immutable baselines/scopes, evidence, reports)
this history entry
```

## Goal and Impact

Nine scoped Windows findings close with assets preserved. Journal writer moves
to format 4, reading 1–3; downgrades require a compatible recovery reader.
Legacy/foreign/malformed evidence remains manual repair. Linux pathname source
publication is still manual review, not an atomic identity guarantee.
No dependency, production reset, real-library mutation, worktree or commit.

## Validation

Final full Rust **1,744 passed/0 failed/29 ignored**, frontend **98 passed**,
native Windows **5/5** (session `6ffe64648026e155b481bfd0fcd34f5f`). Abrupt-exit
receipt reopen and deterministic storage races pass. Build, app/E2E typechecks,
strict all-target Clippy, scoped lint/format, whitespace and fix-report schema pass.
All 25 scoped hashes stayed stable. Independent Windows re-review has no findings.
Failed intermediate runs were retained and corrected, not excluded.

[Full validation and every manual-review boundary](E:/code/projects/EMMM2/docs/plans/mods-manager-hardening/lifecycle-validation.md).
