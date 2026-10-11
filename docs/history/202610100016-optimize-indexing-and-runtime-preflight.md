# Optimize indexing and runtime preflight

## Context

User requested faster onboarding/sync/finishing, accurate progress and simpler flow.

## Changes

- Batch new mod INSERTs with identity in the existing transaction; flush references
  before rebinding. Preserve metadata, safety, existing moves and rollback.
- Prune canonical configured Mods before importer traversal; preserve config parity.
- Report processed mod counts, then indeterminate finishing before cleanup begins.
- Count classified folders after success; clear recheck labels for a new sync pass.
- Remove unused weighted-size/ETA helpers and redundant frontend state.
- Reuse spare root slots for bounded child branches; retain serial recursion.

## Impacted Files

- Rust: `src-tauri/src/modules/library/adapters/sqlite/mods/mutate.rs`;
  `src-tauri/src/modules/reconciliation/application/disk_reconcile/{reconcile.rs,orchestrator/request.rs,projection_writer/mods.rs,projection_writer/write.rs,projection_writer/tests.rs,projection_writer/profile_tests.rs}`;
  `src-tauri/src/modules/system/application/app/{post_apply.rs,runtime_preflight_tests.rs}` (last added).
- Discovery: `src-tauri/src/modules/reconciliation/application/disk_reconcile/{disk_snapshot.rs,census_tests.rs}`.
- Validation fixture compatibility: `src-tauri/src/modules/collections/application/apply/apply_pipeline_tests.rs`
  changes one obsolete constructor to the existing explicit unconfigured API.
- UI: `src/pages/onboarding/{WelcomeScreen.tsx,WelcomeScreen.test.tsx,utils/indexingProgress.ts}`;
  `utils/indexingProgress.test.ts` removed from that onboarding directory;
  `src/widgets/top-bar/GameIndexingOverlay{.tsx,.test.tsx}`;
  `src/shared/api/tauri/events.ts`.
- Translations: `src/shared/i18n/locales/{en,id,zh}/{onboarding,layout}.json`.
- Native QA: `tests/e2e/specs/onboarding-indexing-native.e2e.ts`.
- Docs: `src-tauri/AGENT.md`; `docs/plans/startup-performance/{implementation_plan.md,indexing-efficiency-20261010.md}` (report added); this entry.

## Goal and Impact

Reduce actual redundant work without dependency/schema/IPC fields or concurrency changes.
Synthetic first projection median 439.743→151.383 ms; paired SQL 224.561→30.815 ms;
large preflight snapshot 1,746.774→1.130 ms. Unchanged sync stays within noise.
Paired single-root control 603.623→279.135 ms; retain existing width 8.
Real GIMI excluded. KeyViewer commit-lock traversal remains a separate investigation.

## Validation

1,723 Rust tests and 53 related frontend tests pass. Regression covers batch
boundaries, later-batch rollback/progress, collections/identity, importer digest,
secondary includes and owned Windows junctions. Final strict Clippy/native outcomes
and timing limitations are maintained in the linked report.
Final strict Clippy/full formatting/type/lint pass; fresh native **3 cases / 2
specs pass**. Observed 6,000-mod Home 7.36→3.92 s; discovery 4.61→1.21 s.
Single debug observations exclude cold/p95 claims. Three fixtures removed after
owned shutdown; final QA isolation, source hashes and binary digest verified.

[Measurements and validation](../plans/startup-performance/indexing-efficiency-20261010.md).
