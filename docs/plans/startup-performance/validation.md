# Startup performance evidence

## Final stabilization — 2026-10-10

Stable cached census keys measured an additional 139.595 ms / 10.35% reduction
in complete four-root census, with seven alternating pairs and exact output
parity. Projection profiling found metadata reads dominate; a test-only no-op
UPDATE prototype did not improve complete-stage timing, so no production guard
or read-model shortcut was retained. Native startup hooks now terminate on guard
failure and validate their private identifier before bootstrap.

After correcting invalid dedup fixtures and isolating thumbnail test cache state,
the parallel Rust suite passed 1,614 tests with 21 ignored. A lineage stress test
uses its own expiry window; production TTL and repair guards remain unchanged.
Strict all-target Clippy and E2E types/lint pass. See
[the final stabilization evidence](final-stabilization-20261010.md) for raw
measurement boundaries, source changes and final native results.

## Bottleneck optimization — 2026-10-10

The approved filter now excludes DISABLED ancestors before Home INI harvest.
Real warmed harvest median fell from 1,983.813 to 591.612 ms, retaining exactly
640 effectively active bindings (1,125 → 287 candidate roots). Bounded census
with four roots on existing Rayon measured 1,344.905 ms versus original serial
1,838.443 ms, with exact all-field output parity in five alternating pairs.
These are stage timings, not full startup improvements. Real-root metadata
manifests match before/after; no live app was configured to GIMI this round.
See [the implementation plan and evidence](bottleneck-optimization-20261010.md)
for experiment controls, validation and follow-up profiling decisions.

## Real GIMI audit — 2026-10-10

Three native Windows restarts against 632 GB / 375,099 files passed with a
private read-only audit profile. Home appeared in 0.9–1.3 seconds, core Ready
in 4.4–4.9 seconds and optional keybindings settled in 6.5–12.0 seconds.
Before/after SHA256 manifests and final metadata verification matched exactly;
configuration returned to isolation with zero real-library bindings.
See [the real-library report](real-gimi-readonly-20261010.md) for stage timings,
remaining bottlenecks, disabled-ancestor keybinding work and measurement limits.

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

| Condition      | Samples                                | Median | Row writes                |
| -------------- | -------------------------------------- | ------ | ------------------------- |
| Missing marker | 396836, 404408, 295423, 230387, 223737 | 295423 | At least 5,000 per sample |
| Durable marker | 267, 196, 194, 138, 167                | 194    | 0                         |

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

## 2026-10-09 continuation

The prior preview type errors have been repaired by other work: production build
now passes. Focused startup suites passed 86 frontend tests and 6 native tests.
The repeated backfill benchmark again performed zero writes with a durable marker.

Read-only audit found a larger omitted cost: actual-library watcher registration
took 26.09–28.90 seconds, before the reconcile timer starts. Same-game activation
also discards the already verified startup watcher and rebuilds it. The currently
installed executable retains the old `app_meta` queries and its database has no
marker, so no improved installed-app restart time is claimed.

Full-suite snapshots have failures/warnings in concurrently changed Storage
Optimizer/dedup/driver tests; they are not attributed to startup changes.
See `audit-20261009.md` for measurements, exact validation boundaries and next work.

## Approved implementation — 2026-10-10

Same-game current watcher reuse and pending-owner join are implemented. A bound
owner captures native Mods-root and runtime-parent identities, and only its
current generation may finish or publish. Settings reads use the revisioned
in-memory snapshot; startup shell hydration does not wait for activation ACK.
Explicit switches remain gated. Startup completion cancels stale in-flight
queries before refetching, including no-change results.

Windows registration now caches directory IDs, including dotted/hidden
directories, without prewarming asset-file IDs. Traversal and ID reads use the
existing Rayon pool. Ordered subtree pruning replaces scanning every cached
directory for an asset event. Registration runs in the blocking pool with owned
resources and guarded admission. Cache traversal errors, pre-admission event
loss, ended receivers and native runtime-parent replacement invalidate the
relevant proof; full database repair cannot override missing watcher coverage.

The directory-cache experiment first measured 8,153.657 ms with serial seeding,
then 5,552.621 ms after parallel ID reads. Parallel traversal reduced the same
read-only registration to 1,207.859 ms; three subsequent process runs measured
407.730, 420.264 and 421.969 ms. Every run visited 386,084 entries and cached
30,300 directory identities. The earlier full SDK cache baseline was
26,086.634–28,898.920 ms. These are debug registration-stage measurements with
an uncontrolled/warm OS cache, not OS-cold startup or an established p95.
The optional full SDK control in `startup_watcher_audit` supports a current-build
comparison through `EMMM_AUDIT_FULL_CACHE=1`.

Paired current-build control: native-only registration 0.333 ms, full SDK cache
40,871.037 ms, directory-cache production registration 517.780 ms. Both SDK
policies used the same root sequentially in one process; production still
visited 386,084 entries/cached 30,300 directories. This confirms a registration
cost reduction beyond the variation of earlier samples, without asserting a
whole-startup percentage or controlled OS-cold result.

Object-key backfill now matches the normal object namespace. Native fixture
counters changed from two identity key value changes to zero on the next
unchanged reconcile, with one index load. Object-only stale-key repair loads
two indexes instead of three. Real mod swaps still reload; collection-reference
tests remain covered. This does not claim zero SQL UPDATE statements.

Focused behavioral regressions reproduced and repaired: obsolete publication
duty terminating a live receiver, invalid-owner read waiters remaining pending,
runtime-parent replacement reusing old coverage, completed-owner joins after
receiver end, inactive source repair dropping the selected watcher, and a raw
onboarding change escaping precomputed projection while the journal stayed
unchanged. Native INI/asset moves verify both root sizes and current authority.
Existing mod rows intentionally remain while their physical folders exist;
the experiment does not change that deletion contract.

Frontend validation: 68 focused tests; the whole workspace suite passed 1,264
tests with one skipped. Production build, TypeScript including the E2E config,
targeted ESLint and architecture lint passed. Whole-workspace ESLint had no
errors and five formatting warnings in concurrently edited driver tests.
Whole-workspace counts include other changes and are not attributed solely to
startup work. Current SDK control is recorded above; final native suite and
isolated restart measurements follow.

### Native application restarts

`wdio ... --spec tests/e2e/specs/startup-reopen.e2e.ts` passed on the newly
rebuilt debug application using verified identifier `com.reynalivan.emmm.e2e`
and an owned 300-mod fixture. Each of three actual process restarts had exactly
one watcher registration, one full `StartupBoot` baseline and zero full
`GameSwitched` scans. Configuration persisted and all 300 mods were readable
after core readiness settled. Owned native processes stopped before fixture
cleanup.

Dashboard DOM observation page-clock samples were 939, 795 and 943 ms after
navigation. The harness navigates the fresh WebView and waits for its IPC
bridge before assertions. These samples exclude driver/process creation and
are not first-paint timings, a p95, or large-library end-to-end startup timings.
An earlier harness attempt queried IPC before fresh navigation and was fixed;
the final restart assertions passed.

Final focused watcher suites passed 117 tests in three consecutive parallel
runs (4.43, 4.79 and 5.27 seconds). The native application build and E2E
TypeScript validation passed. An obsolete coverage test's fictitious runtime
paths were replaced with real owned parent directories, preserving the stronger
native identity contract.

The latest whole-library run passed 1,504 tests, failed four and ignored 15.
Three owned failures exposed test timing assumptions under parallel load:
raw-snapshot SDK delivery now uses a test-only mutex/condition-variable gate,
and both native file-move cases capture the real baseline watermark and use
bounded catch-up. All three passed in the repeated 117-test suites. The remaining
failure is the concurrently added `discard_catalog_import` command missing its
permission in `permissions/app-commands.toml`; unrelated Catalog edits were
preserved. The whole-library suite is therefore not reported as passing.

Final whole-target Clippy passed with `-D warnings`, including the test-stability
changes. A preceding repeat encountered transient compile errors in concurrently
edited `library/application/mods/preview_ops.rs`; those edits were preserved and
the successful check ran after that file was corrected by its owner. Final
documentation formatting and `git diff --check` also passed.

### Follow-up: routine sizing and deferred staging cleanup

Approved priorities 1–2 remove recursive sizing from ordinary discovery and
reconciliation, including StartupBoot and watcher changes. The optional size
projection, dedicated known-path sizing query and size update helper were
removed. Existing sizes remain unchanged; newly indexed mods use the existing
schema default. Explicit Storage Optimizer scan measurement remains intact.
Dashboard waste now comes from its persisted measured report bound to the
configured root, rather than stale `mods.size_bytes`.

Startup validates import references and atomically detaches terminal/orphan
garbage into the owned `.cleanup` quarantine before import commands can run.
The native `Ready` worker deletes only quarantine payloads through
`spawn_blocking`, never enumerating new live staging attempts. Interrupted
cleanup leaves quarantine for the next launch. Preparation still performs
shallow layout reads/renames; recursive deletion is outside setup. This is an
event-loop boundary, not a measured first-paint boundary.

The extended native E2E passed one test covering three process restarts on a
rebuilt 300-mod application fixture. Each restart detached one isolated orphan,
deleted it in the `Ready` worker, registered one watcher and performed one
StartupBoot baseline with zero GameSwitched scans. Dashboard page-clock
observations were 812, 823 and 746 ms; these exclude driver/process creation,
are not OS-cold or p95 samples, and do not establish a causal percentage gain.
Production build, E2E TypeScript, targeted ESLint and native cargo check passed.
Reconciliation passed 211 tests (two manual benchmarks ignored); native file
move tests passed two; staging cleanup passed six behavioral tests.

The dashboard suite passed ten tests after review reproduced a deleted indexed
duplicate still appearing in measured JSON (500 bytes versus expected zero).
The final query checks surviving same-game group membership before selecting
the first surviving primary. Unindexed members remain supported; external
deletion of an unindexed member is only known after another explicit scan.
Waste is a last-scan estimate, not a continuously measured total.

A local Windows temporary-fixture comparison used 1,000 files in 50 directories
per terminal batch, three paired samples. Previous recursive startup deletion
took 85,511 / 102,711 / 96,707 microseconds (median 96.707 ms). New startup detach
took 8,593 / 5,979 / 6,629 microseconds (median 6.629 ms). Deferred deletion took
97,366 / 84,835 / 91,684 microseconds (median 91.684 ms). Payload creation was
outside timing; these uncontrolled-cache debug helper timings show work moved
out of setup, not faster total deletion or large-library end-to-end startup.
The ignored manual benchmark passed. Native review found no remaining actionable
finding after the deleted-member correction.

Final whole-library run: 1,537 passed, one failed, 16 ignored. The remaining
failure is `whitelisted_exact_pair_is_not_reintroduced_through_a_third_duplicate`
in the concurrently modified duplicate scanner tests; running it alone still
fails (empty groups versus the expected two groups). This follow-up did not
edit duplicate scanner code/tests. The whole-library suite is not reported as
green. Final whole-target Clippy with `-D warnings`, scoped Rustfmt, E2E
TypeScript/ESLint, documentation formatting and diff whitespace checks passed.

### Follow-up: optional Home keybindings and task-log retention

Priorities 3 and 5 were approved on 2026-10-10. Home starts optional INI reads
only after the existing selected-game activation reaches Ready, with no pending
activation request. Query identity includes activation generation. The widget
shows a reduced-motion-compatible accessible skeleton during deferral/first
fetch; other Home cards remain visible. Error/unavailable/removed-game states
settle visibly. Fixture demo reads have an explicit mode exemption because no
native activation exists there; production still observes the readiness gate.

Task-log retention moved out of setup into the existing background startup
worker after selected-game recovery settlement. Its SQL policy is unchanged:
old settled rows may be deleted, while Pending/Running rows and newer settled
rows survive. A migrated-schema regression passed, including repeated cleanup.
Earlier fixture attempts violated existing game/path and open-task uniqueness;
fixtures were corrected without changing production constraints.

Frontend: 34 focused tests passed, including real React Query stale-response
cases, pending same-game activation, demo fixture/no-native-IPC behavior, widget
skeleton/error and other Home content remaining visible. Independent review's
demo infinite-loading finding was fixed and reviewed again.

Final native E2E passed one test with three actual process restarts of a rebuilt
300-mod application fixture. Each restart displayed the fixture's F1 keybinding
in Home, ran task cleanup after StartupBoot timing, retained one watcher/one
baseline/zero duplicate GameSwitched scans, and completed deferred staging
cleanup. An initial assertion selected the hidden mobile duplicate of the key
at desktop width; the final assertion checks all candidates for actual display.
Page-clock Home observations were 1,144 / 1,009 / 886 ms, excluding driver/process
creation; no total-startup percentage, p95 or large-library result is claimed.

A demo browser was briefly inspected before the user requested native-only E2E;
its development server was stopped, and final integrated verification was native.
Production build and strict whole-target Clippy passed. The prior whole-library
duplicate-whitelist failure remains a known boundary; this scoped follow-up did
not edit that scanner and did not repeat the full suite.

Final E2E TypeScript, scoped ESLint, architecture lint, strict whole-target
Clippy, scoped Rustfmt, documentation formatting and `git diff --check` passed.
