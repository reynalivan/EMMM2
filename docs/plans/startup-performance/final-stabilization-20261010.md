# Final startup stabilization and measured follow-ups

User approved closing build/Clippy issues, measuring cached census sort keys,
profiling projection and retaining only meaningful improvements, then validating
the native Windows application on a fresh build.

1. Resolve strict lint findings without weakening checks or changing ownership
   validation. Preserve unrelated in-flight edits; run all-target Clippy.
2. Compare stable comparator sorting and cached keys on the same captured census
   order, including equal keys, Unicode and all entry fields. Compare complete
   four-root census too; keep only a repeatable end-to-end stage improvement.
3. Profile index loading, identity work, object/mod passes, metadata reads,
   pruning, read-model refresh and commit separately. Use a writable isolated
   copy of the archived audit DB; GIMI is only read. Count redundant table writes
   and test any proposed guard before applying it. Do not skip validation,
   metadata discovery, recovery or read-model repair to manufacture a speedup.
4. Review retained changes; run relevant Rust/frontend checks and freshly built
   native Windows restart tests with real watchers and owned fixtures. Record
   build identity/artifact and remaining limitations. Production installation
   and changes to the real library are outside this task.

## Safety and measurement boundary

No real GIMI app binding, file mutation, rename, cleanup or deletion. Root metadata
manifests cover the read-only benchmarks. All database mutations remain in a
workspace/temporary clone. Prefer existing Rayon and SQLite interfaces; add no
dependency, persistent cache or service. Keep experiment commands, exact output
parity, samples and retained/rejected conclusions in the validation ledger.

## Results

Initial strict all-target Clippy passes after three logically equivalent
`is_none_or` rewrites in Trash/Recovery; no validation condition was removed.
Remaining experiments and final native evidence will be recorded below.

### Census: retained

Seven alternating pairs on the same captured 30,290-entry pre-sort order:
allocating comparator median 162.174 ms (160.846–164.773), cached keys 22.815 ms
(22.207–23.678). Complete four-root census median 1,349.361 ms
(1,325.670–1,376.034) → 1,209.766 ms (1,202.859–1,220.265), 10.35% / 139.595 ms
less time. All four fields and stable case-equal ties match exactly. The three
census call sites share a private standard-library cached-key sorter; keys are
temporary and discarded after sorting. No persistent cache or worker change.
Sixteen disk-snapshot tests pass, three manual tests ignored by default.
Raw samples: `.audit-output/gimi-final-stabilization-20261010/census-sort.log`.

### Projection: measured; guard prototype rejected

The archived matched projection contains 148 objects / 4,243 mods. Its writer
and read-model refresh run against a consistent workspace DB clone; actual mod
metadata is only read. Phase metrics separate initial index, identity work,
object/mod passes, metadata, finalization, read-model refresh and commit.
Metadata time is included in the mod pass, and any subsequent index reload is
included in identity time; do not sum the overlapping metadata field.

The first access measured 3.15 seconds of metadata reads, while subsequent
warmed reads were roughly 0.29–0.43 seconds. Read-model refresh was roughly
7–8 ms. An unchanged pass performs 592 object UPDATEs and deletes/reinserts 148
read-model rows; mod UPDATEs settle to zero after warm-up.

A test-only BEFORE UPDATE trigger suppressed only wholly unchanged object rows.
Seven alternating pairs measured total median 471.595 ms without suppression
(454.385–588.107) versus 483.340 ms with suppression (455.206–564.339). Five
of seven suppressed runs were slower; object-pass medians were 26.457 / 24.396 ms.
The ranges overlap and there is no repeatable total improvement. No production
SQL guard was added. The 7-ms read-model rebuild remains authoritative for
repairing derived data; no startup validation or metadata read was skipped.

This clone benchmark includes counter/prototype trigger overhead, uses persisted
projection entries and excludes discovery, conflict/rename preflight and UI/IPC.
It is not whole-startup timing or proof of an equivalent optimized WHERE clause.
Raw full trace: `.audit-output/gimi-final-stabilization-20261010/projection-profile.log`.

### Validation safety

Review caught and corrected two pre-execution harness hazards. The profiler now
checks canonical containment in both directions and rejects ancestor/file/SQLite
sidecar links/reparse points before opening its owned DB. Owned overlap and real
Windows ancestor-junction fixtures pass without opening SQLite on the fixture.

WDIO logged hook exceptions and still sent four session requests to a controlled
fake driver after preparation was rejected. Terminal catches now log the original
error, attempt only owned teardown/restoration and exit even if cleanup fails.
The same regression now exits with zero session requests. Audit restoration
failure also fails the run. A private expected-identifier environment contract is
checked at the existing first bootstrap guard, before settings/recovery, so a
shared-target wrong-profile binary cannot use production configuration. Normal
launch without that E2E contract retains its behavior. Seven guard tests and nine
driver lifecycle tests pass; E2E types/lint pass.

### Integrity and full-suite stabilization

Before/after metadata inventories match exactly for all 375,099 files / 30,317
directories / 632,024,978,228 bytes; both manifest SHA256 values are
`09D200FB0D4504CFEB73AFCA1E1AFF36864A400B0CD7073079250E19002D4CEA`.
This phase did not repeat the prior full-content checksum. All GIMI operations
were reads; no app was bound to it. Data writes belong only to the managed clone.

The first full Rust run passed 1,611, failed two and ignored 21. The dedup whitelist
regression used an invalid mod INI under the current strict classifier; adding
the existing valid INI fixture and explicit candidate counts restores a meaningful
passing regression. No production duplicate grouping change was needed.
The thumbnail decoder passed independently; failure came from unrelated tests
resetting its global cache. Resolving tests now use explicit owned cache handles
through the same engine, preserving the production singleton, reset/visibility
guards, semaphore and three retries. Eight tests pass, including generation
invalidation during hiding and independent-cache reset isolation. Source review
found no actionable defect. Unrelated-folder cancellation in production remains
unchanged and is a separate preview-domain follow-up.

The next full run passed 1,613, failed one and ignored 21: the 1,000-rename lineage
stress exceeded the ten-second evidence TTL under parallel load. It passed alone.
Only this test's evidence window is extended to five minutes, separating lineage
stress from existing explicit expiry tests. Production TTL/repair remains
unchanged. All 25 suppressor tests pass. Final full-suite/native evidence follows.

Final full Rust library suite: 1,614 passed / zero failed / 21 ignored in
87.26 seconds. Strict all-target Clippy with warnings denied passes after all
retained code/test changes. Full E2E TypeScript, focused ESLint/format and
whitespace checks pass. Both E2E app profiles have zero protected-library
bindings before the final fresh-build native fixture launch.

### Final native build

Fresh frontend TypeScript/Vite production assets and native debug E2E executable
built successfully with identifier `com.reynalivan.emmm.e2e`. The native Windows
startup spec passed one test covering three actual process restarts on 301 owned
mods: visible active F1, no disabled-ancestor F12 in IPC/Home, one watcher and one
StartupBoot baseline per restart, zero full GameSwitched baselines, and deferred
maintenance settlement. Page-clock Home observations: 1,020 / 1,004 / 736 ms.
These small-fixture observations exclude driver creation and are not real-library
whole-startup or cold-cache timings.

Owned native process-tree shutdown and fixture cleanup completed. No browser demo
was used; no production install/config was changed. The final isolated binary is
`src-tauri/target/debug/emmm-e2e.exe`; its identity/hash are retained in
`.audit-output/gimi-final-stabilization-20261010/native-final-build.json`.

The default tauri-driver run emitted its existing child-process `shell: true`
deprecation warning. It did not affect the passing assertions; no dependency or
driver implementation change was bundled into this startup optimization.

### Follow-up architecture cleanup and thumbnail cancellation

The separate preview follow-up noted above is now resolved in a scoped change.
One Workspace domain predicate replaces duplicate effective-enable policies;
projection fields/logs explicitly name `initial_index` and
`metadata_within_mods`. The existing bootstrap guard entry point is renamed
without changing its ordering or protections.

Unrelated folder invalidation and removed-source junction alias scenarios both
reproduced before correction. Live resolution tickets now scope cancellation to
physical/overlapping canonical folder identities; missing suffixes resolve via
the nearest existing ancestor. Tickets are removed on every exit. The existing
four-permit semaphore remains owned by the blocking closure after async caller
cancellation, while full reset still invalidates all work. No persistent
per-folder epoch map, new worker or public API was added.

Final follow-up gates: 1,635 Rust tests pass / zero fail / 21 ignored, including
16 thumbnail tests; strict all-target Clippy, full Rust formatting, whitespace
and E2E TypeScript pass. Fresh native Windows debug validation passes five cases
in two specs: three startup restarts and four gallery/visibility cases. All owned
processes and three fixtures were cleaned; image bytes remain unchanged. No GIMI
access or production install/config change occurred in this follow-up. The
existing child-process deprecation warning remains. Async cancellation admission
has source-level review rather than a dedicated stress regression.

Full record and validated binary hash:
[`cleanup history`](../../history/202610100012-simplify-startup-and-thumbnail-invalidation.md).
