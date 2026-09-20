# Faster, truthful first-run indexing

## Context

The onboarding progress screen stayed at a fabricated low percentage while preflight performed the most expensive filesystem work. This made the interface look stuck and hid the distinction between preparation and reconcile apply.

## Changes

- Merge onboarding metadata collection and the directory identity census into one filesystem traversal.
- Replace per-file ancestor byte accumulation with direct directory totals and one post-order roll-up.
- Reuse that census in onboarding projection, avoiding a second directory traversal.
- Emit throttled preparation progress for metadata and classification, followed by explicit ready and recheck states.
- Show phase-specific counters and determinate root progress only when a backend denominator exists. Do not show a global percentage or ETA during preflight.
- Record opt-in anonymous aggregate durations for preparation, classification, apply, and fallback recheck. Events contain only operation, outcome, error category, and duration.

## Impacted Files

- `src/pages/onboarding/WelcomeScreen.tsx` (modified)
- `src/pages/onboarding/WelcomeScreen.test.tsx` (modified)
- `src/shared/i18n/locales/en/onboarding.json` (modified)
- `src/shared/i18n/locales/id/onboarding.json` (modified)
- `src/shared/i18n/locales/zh/onboarding.json` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/disk_snapshot.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs` (modified)
- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs` (modified)
- `src-tauri/src/modules/system/application/telemetry/mod.rs` (modified)

## Goal

Make first-run indexing faster and visibly truthful from the first event, while keeping filesystem correctness and the full-reconcile fallback intact.

## Impact

No filesystem scan cache or cross-drive parallelism is introduced. The existing anonymous diagnostics setting controls the new aggregate telemetry; paths, game names, counts, and file contents are never recorded. Reduced-motion behavior remains supported by the spinner and progress-bar guards.

## Notes

## Measurement

The checked-in synthetic fixture validates semantics, not a production timing claim. A representative cold library was not available in this workspace, so no baseline/result is recorded here.

Before rollout, benchmark the old and new preparation path separately on the same cold library for five runs. Retain the optimization only when the median preparation time improves by at least 10% and p95 does not regress. Record the hardware, library shape, five raw durations, median, and p95 in this note.
