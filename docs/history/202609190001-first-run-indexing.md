# Faster truthful first-run indexing

## Context

First-run indexing spent significant time in unreported filesystem preflight work and displayed a misleading global percentage.

## Changes

- Merged onboarding metadata and identity census discovery, then rolled directory sizes up once after traversal.
- Added throttled Metadata, Classifying, Ready, and Rechecking progress events with live counters.
- Replaced the synthetic global percentage and early ETA with phase-specific progress and counters.
- Added opt-in aggregate duration telemetry for preparation, classification, apply, and fallback recheck.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/disk_snapshot.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs` (modified)
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/types.rs` (modified)
- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs` (modified)
- `src-tauri/src/modules/system/application/telemetry/mod.rs` (modified)
- `src/pages/onboarding/WelcomeScreen.tsx` (modified)
- `src/pages/onboarding/WelcomeScreen.test.tsx` (modified)
- `src/shared/api/tauri/bindings.ts` (modified)
- `src/shared/i18n/locales/{en,id,zh}/onboarding.json` (modified)
- `docs/history/202609150002-onboarding-progress-estimate.md` (modified)

## Goal

Indexing is faster on cold start and remains visibly active without inventing progress values.

## Impact

Filesystem semantics and fallback rechecks remain intact. Telemetry records no paths, game names, counts, or file contents.

## Notes

Representative cold-library benchmark figures remain pending because this workspace has no representative library fixture.
