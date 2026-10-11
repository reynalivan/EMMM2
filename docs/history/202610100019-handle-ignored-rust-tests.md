# Handle ignored Rust tests safely

## Context and changes

User requested handling the 27 ignored library tests and approved synthetic data.
Classified manual benchmarks, external-input tests and crash worker helpers.
Added a fresh owned fixture generator and UUID-bound projection profiler mode;
kept legacy data/guard assumptions intact and avoided real libraries/DBs.

## Impacted files

- `scripts/prepare-ignored-audit-fixture.mjs` (new).
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/projection_writer/profile_tests.rs`.
- `.bug-hunter/ignored-tests/{capture-snapshot,run-selected,finalize-run}.cjs`
  and canonical coverage evidence; owned `.audit-output/ignored-tests/` records.
- `docs/plans/mods-manager-file-locks/ignored-tests.md` and this history entry.

## Goal and verification

Original 27 handled: 24 direct opt-in PASS, three worker entry points covered by
passing parent subprocess harnesses. Four profiler guards and runner root-escape
rejection pass; Node/ESLint/format checks pass, independent review has no finding.
Input DB unchanged; integrity/FK checks pass. Owned temp fixture/scratch removed,
regeneratable data only; logs, DBs and executable retained.

## Impact and limits

Default ignore markers remain intentional. Snapshot contains 29 ignored tests
after two concurrent manual benchmarks were added; the owner covers those.
No production fix, dependency, schema migration, task override or user process
termination. Frozen debug results do not certify later optimizer source edits.

[Inventory and executed evidence](../plans/mods-manager-file-locks/ignored-tests.md).
