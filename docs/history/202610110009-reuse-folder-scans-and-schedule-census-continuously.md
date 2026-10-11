# Reuse folder scans and remove census batch waits

## Context

The user approved measured startup optimizations without caches, dependencies,
overengineering or writes to the real GIMI library.

## Changes

- Reuse strict FolderScan children in normal classification; preserve missing/file
  SourceUnavailable errors and inclusion policy.
- Schedule census on at most four scoped Rayon workers per pass. Ordered channel
  merging preserves online progress and earliest indexed errors; workers join
  before return. Empty/single-root or width-one walks stay serial.
- Add opt-in native WebView2 CPU/script/layout/style/task diagnostics with cleanup
  on failed startup. UI optimization awaits a usable desktop and profiling evidence.

## Impacted Files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/modules/reconciliation/application/disk_reconcile/census_tests.rs`
- `tests/e2e/support/nativeCpuProfile.ts` (added)
- `tests/e2e/specs/startup-real-library-readonly.e2e.ts`
- `src-tauri/AGENT.md`
- `docs/plans/startup-performance/scan-reuse-census-ui-20261011.md` (added)
- This history (added)
- Ignored `.audit-output/startup-next-20261011/`: fixtures, snapshot/CPU/benchmark
  helpers, manifests, contract checks, logs and measured evidence.

## Goal and Impact

Seven alternating release pairs: GIMI classification 214.715→205.321 ms (4.38%,
6/7 faster); container-rich fixture 174.713→140.968 ms (19.31%, 7/7). Production
census 900.899→402.092 ms (55.37%, 7/7), exact 30,290 entries and 149 progress events.
These are warmed stage timings, not full startup or cold-storage certification.
No schema/dependency/API changes, custom scheduler or persistent pool cache.

## Verification and Limits

RED proved the old batch barrier; GREEN passed 12 scheduler cases. Reconciliation
passed 287 tests with 15 manual cases ignored; focused review found no issue.
Strict all-target readonly-audit Clippy, E2E types/lint, scoped formatting and diff
checks passed. A cleanup-owner needless-return warning was fixed in its own history.
GIMI metadata/structure still matches the earlier full checksum manifest; no full
content rehash this round. Four readers is a per-pass cap. Fresh private debug
native normal-sync/KeyViewer (3 cases) and startup/reopen (1 case) passed through
the coordinated run; visible UI profiling remains blocked by the locked desktop.
