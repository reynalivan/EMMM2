# Optimize Home keybindings and folder census

## Context

Real GIMI startup audit found unnecessary disabled-ancestor INI harvests and
serial folder census. User approved fixes, a plan and bounded parallel benchmarks.

## Changes

- Dashboard candidates reuse Library's effective-enable predicate through its
  API; DISABLED ancestors are filtered before INI discovery without status writes.
- Census processes at most four roots together on existing Rayon; the original
  DFS, sorting, inclusion/errors and ordered progress prefixes remain intact.
- Add regression/parity tests, read-only comparison tools and a disabled-ancestor
  native restart fixture. Record the plan and measured trade-offs.

## Impacted files

- `src-tauri/src/modules/library/api.rs`;
  `src-tauri/src/modules/library/adapters/sqlite/mods/listing.rs`.
- `src-tauri/src/modules/dashboard/adapters/sqlite/dashboard/{mod,tests}.rs`.
- `src-tauri/src/modules/system/application/app/dashboard.rs`;
  `src-tauri/src/modules/system/application/app/tests/dashboard_benchmark.rs` (new).
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/disk_snapshot.rs`;
  adjacent `census_tests.rs` (new).
- `tests/e2e/specs/startup-reopen.e2e.ts`; `src-tauri/AGENT.md`.
- `docs/plans/startup-performance/bottleneck-optimization-20261010.md` (new);
  `docs/plans/startup-performance/validation.md`; this history.

## Goal and impact

Warm real-library census median: 1,838 → 1,345 ms (26.8% less time); all census
fields match. Warm INI harvest median: 1,984 → 592 ms (70.2% less time), candidates
1,125 → 287, retaining all 640 effectively active bindings exactly. Neither
percentage represents full startup. Progress is now emitted after each batch.
No dependency, migration, persistent cache or new thread pool. Real benchmarks
only read GIMI; metadata manifests match and app profiles remain isolated.

## Validation

Filter regression reproduced before correction. Dashboard/service: 17 passed;
reconciliation: 230 passed (three ignored, including manual census benchmark).
Both manual benchmarks pass; source review, scoped formatting, E2E TypeScript
and ESLint pass. Fresh frontend/native build and one native test with three
301-mod restarts pass; active F1 retained and disabled-ancestor F12 excluded.
Strict Clippy first found unrelated warnings; its post-native retry was blocked
by a concurrent hotkey safe-mode signature mismatch (E0061). Files were preserved.
Details, earlier driver-port collision and measurement limits are in the plan.
