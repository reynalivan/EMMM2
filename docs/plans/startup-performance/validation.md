# Startup performance evidence

## Retained changes

- Unicode backfill version uses existing `app_settings`; marker check, reads,
  writes and publication share one immediate SQLite transaction.
- Router boot runs once across StrictMode replay/navigation. Overlapping
  `initStore` calls share one operation; completion/failure clears the promise.
- Existing recovery, activation, watcher-authority and UI readiness gates remain.

## Regression evidence

Before the marker fix, three native regressions failed: version absent after
backfill, repeated startup made four additional writes on the small fixture,
and injected version-write failure was silently ignored. After the fix all
three passed, with zero writes on reopened startup and rollback on failure.

Frontend regressions observed two recovery checks during StrictMode replay and
three settings reads during overlapping initialization. Both now execute once.
The focused final frontend run passed 85 tests across four files. The full
frontend suite passed 1,215 tests, with one skipped; concurrent changes are
included in that whole-workspace count, not attributed to this fix.

## Backfill measurement

Run the ignored native `benchmark_reopened_index_backfill` test. Its 5,000-mod
SQLite fixture measures the same production backfill on one database with a
missing marker versus a durable marker. Marker deletion is outside timing.
Five alternating samples, in microseconds:

| Condition | Samples | Median | Row writes |
| --- | --- | --- | --- |
| Missing marker | 396836, 404408, 295423, 230387, 223737 | 295423 | At least 5,000 per sample |
| Durable marker | 267, 196, 194, 138, 167 | 194 | 0 |

This is an in-memory backfill-stage comparison, not a measured configured-app
restart. The earlier user log's full restart reconcile was 9,936 ms. Full disk
observation after reopening remains; no new whole-startup time or classification
speedup is claimed. Production database/library were not modified for validation.

## Discarded experiment

Nonblocking settings hydration plus a readable syncing dashboard were restored
to their original behavior before completion. Focused review found an overlapping
activation could replace pending boot recovery and submit another scan; initial
in-flight dashboard queries could also retain stale data after invalidation.
Fixing those would require additional lifecycle work beyond the smaller verified
backfill/initialization fix.

## Workspace build boundary

Full ESLint and architecture lint passed. `pnpm build` was attempted twice and
stopped at three TypeScript errors in concurrently modified
`src/widgets/mod-preview/hooks/usePreviewPanelState.test.ts` (lines 103, 240,
247: incomplete query/mutation mocks). Those unrelated changes were preserved.
Native suite, formatting and Clippy results are recorded below when complete.
