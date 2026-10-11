# Watcher refactor plan audit

Date: 10 October 2026. Scope: design and migration plan only.
Verdict: revised plan is suitable for implementation approval, subject to a fresh
source baseline and the listed acceptance gates. No source implementation or
runtime certification was performed in this task.

The backend architecture reviewer independently checked the previous plan
against current source. Parent review mapped integration callers and test
migration, and strengthened the facade compatibility rule. These are gaps in
the old plan, not six newly confirmed production bugs.

## Findings and disposition

| ID     | Priority | Old plan gap                                                                                                                      | Correction in refactor_plan.md                                                                                                                         |
| ------ | -------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| PLAN-1 | P1       | One event disposition could lose ordered rename sides.                                                                            | Original event retained; separate authority/content delivery facts per endpoint; suppression follows observation.                                      |
| PLAN-2 | P1       | Helpers called pure could read filesystem metadata/native identity.                                                               | Explicit I/O evidence acquisition and pure comparison; final admission recheck remains stateful.                                                       |
| PLAN-3 | P1       | All runtime enqueue was moved after lease release.                                                                                | Ordinary batch and activation Ready publication preserve their distinct existing lease/authority windows.                                              |
| PLAN-4 | P1       | Receiver cancellation and onboarding raw lease/claim ownership were underspecified.                                               | One consumer and actual session; outer RAII retirement survives every exit; watcher-before-scan, raw lease, claim, repair and drain ordering retained. |
| PLAN-5 | P2       | Equivalent bindings could be confused with actual session health/Notify, or all modes could require newest allocation generation. | Binding evidence separated from real resource; candidate/active/inactive mode checks preserve valid older prewarmed resources.                         |
| PLAN-6 | P2       | Facade forwarding was assumed to remove dependency cycles.                                                                        | Directed activation -> runtime -> core; atomic caller migration; no application shim forwarding back into a Tauri adapter.                             |

All six are addressed in the revised document. Acceptance tests are requirements,
not completed checks. The review does not authorize broad implementation.
Final independent document review found no material plan blocker; native timing,
admission races and cancellation remain gated by fresh implementation tests.

## Source evidence

Paths below are under `src-tauri/src/`; `watcher/` abbreviates
`modules/workspace/application/scanner/watcher/`:

- PLAN-1: `modules/workspace/application/scanner/watcher/mod.rs`, stitched rename
  classification around 429-445; suppressor's original-event echo matching.
- PLAN-2: `watcher/event_filter.rs` content branch calls `path.is_dir()`;
  `watcher/suppressor.rs` covers/runtime-parent comparison reads native identity;
  `watcher/registration.rs` binding capture/admission. Snapshot comparisons are
  not perpetual ownership proof.
- PLAN-3: `watcher/lifecycle.rs` ordinary batch around 2372 onward versus
  `CurrentActivationReuse` 217-235, activation publication 1086-1199 and acceptance
  guard 1824-1944. Parent reread confirms enqueue occurs before activation guard
  release, under current-activation publication authority.
- PLAN-4: `watcher/lifecycle.rs` receiver lifetime 87-112 and initial
  recovery/onboarding around 1598-1790. Latest onboarding history preserves
  scoped live owners, raw watcher-before-discovery and per-attempt progress.
- PLAN-5: `watcher/suppressor.rs` equality excludes validity/Notify Arcs;
  candidate/current-reuse comparison construction is not a new real resource.
- PLAN-6: `workspace/api.rs`, `workspace/adapters/tauri/watcher_activation.rs`;
  reconciliation orchestrator consumes watcher state while lifecycle already
  consumes reconciliation. A facade controls access, not graph acyclicity.

The exact source hashes are in `plan_audit_source_snapshot.json`. CodeGraph
auto-sync was disabled; current files and scoped repository searches were used.
This was a focused symbol/contract review, not complete codebase bug coverage.

## Caller inventory checked

Entrypoint searches found production consumers in settings Tauri commands,
reconciliation Tauri disk commands and system bootstrap. The untracked existing
watcher_activation adapter was read directly. The plan explicitly includes
workspace facade and module registration changes alongside those consumers.
Unrelated application architecture debt is excluded.

## Test evidence mapped, not rerun

Existing source contains:

- Five `watcher_fix_*` regressions for sibling noise, suppressed wake, coalescing,
  expiry ownership and invalid inactive cache.
- Receiver cancellation/successor recovery retirement tests in lifecycle.
- Native candidate admission/root replacement and blocking-registration tests.
- Dirty/clean handoff, authority generation and inverted-lock reconcile tests.
- Native watcher stress and startup-reopen fixtures from the preceding fixes.

Required additions: complete ready-idle loop -> DB/result convergence; ordered
rename/one-side suppression; compare-only binding versus real unhealthy resource;
mid-admission invalidation; cancellation across all owning stages; separate
activation-versus-batch publication ordering. Existing signal/startup tests alone
cannot establish those guarantees.

## Verification for this documentation task

Check document formatting, JSON shape, local links/path inventory, plan/implementation
status consistency and unchanged source hashes. Application builds, tests and
native E2E are deliberately not rerun: no application source is changed. Previous
test counts are historical evidence only, not a new validation result.

## Residual risks

Native Windows timing and external filesystem replacement cannot be made atomic
by a pure snapshot. Retain final validation, dirty-generation continuity and
cache-invalidity rebuild. Freeze the exact source scope before each implementation
checkpoint; shared-checkout drift requires a fresh read, not assumed compatibility.
