# Final startup, indexing and runtime audit

Date: 2026-10-11. Status: progress fixes implemented; four release fixture
benchmarks, 15 native release cases and four readonly startup launches passed.
Zero-loss WPR locates a sampled frame gap in renderer execution; attribution to
specific application functions remains open.

## Execution checkpoint

- The listener now ignores callbacks after disposal and rejects a lower count
  within the same run, phase and total. Terminal events reset its single previous
  payload, preserving the valid same-ID authority retry.
- Runtime toast labels scanning in folders and projection in mods. Unknown/zero
  totals and finishing use an indeterminate indicator. Existing translations,
  short-operation delay and recovery behavior are retained.
- Focused regression tests went from 7 failing assertions to **16 passing tests**.
  Application typecheck, scoped ESLint and formatting checks passed. Independent
  review found no actionable defect in the four changed frontend files.
- Four opt-in, owned-fixture Rust experiments passed in release: dispatch
  amortization, bounded dispatch, onboarding batch holds and foreground/cancel
  latency. These remain test-only additions; production scheduling is unchanged.
- WPR request/acknowledgement files are published atomically and validated by
  run token and phase. After two lossy captures, the focused retry completed
  with **zero lost events/buffers** and four successful native launches.
- Fresh WPR-fixture baseline contains 4,248 files and 4,395 directories, with
  digest `59F14172DAA27661C6BC2D012054D580BF85925827FEC9A21FB2A19C66B9A72B`.
  This implementation pass has not accessed the real GIMI library.

Cargo, shared generated bindings, dist and native execution were coordinated with
the hotkey and Dashboard/preview tasks. Final source/compiler/native ownership
was returned to the cleanup task after verified shutdown and binary preservation.

## Release fixture measurements

Build: optimized Rust library tests, 2m59s. All four unique ignored tests passed
serially; **695 Rust inputs had zero drift** through the run. Executable SHA-256:
`7e5be87beb85ad0f01d7610b0ca02b9d0c2598be2796d76abcf13fef6d6d0839`.
Raw logs, environment and paired statistics are in
`.audit-output/final-performance-audit-20261011/release-bench/`.

Hardware: i7-13700K, 16 cores/24 logical processors, about 32 GB RAM. Genshin and
other Cargo/rustc jobs were absent at the measurement-start snapshot. Desktop
load was otherwise uncontrolled; a single CPU counter snapshot is not a continuous
load record. Filesystem cache was warm/uncontrolled; cold below refers only to the
parsed-harvest cache. These are stage measurements, not app startup or GIMI results.

### Harvest dispatch

Five alternating paired runs per mode/cache/scope; 2,048 fixture mods, one 3,920-byte
INI each. Batches execute serially with **one in-flight blocking task**, not a new
pool. Exact output ordering, targets, keybindings, fingerprints, INI-read counts
and cache identity checks all passed.
The difference includes path/capability batching costs as well as task dispatch;
it is not an isolated measurement of Tokio scheduler latency.

| Full scope / parsed cache | Current per-mod |   Batch 2 |   Batch 4 | Batch 4 median reduction |
| ------------------------- | --------------: | --------: | --------: | -----------------------: |
| Warm                      |       342.62 ms | 302.87 ms | 291.78 ms |                   14.84% |
| Cold                      |       407.60 ms | 379.01 ms | 359.24 ms |                   11.86% |

Batch 4 won all five full-scope pairs in each cache condition. Median paired savings
were 49.75 ms warm (MAD 13.50 ms) and 44.92 ms cold (MAD 3.71 ms). Batch 1 showed no
consistent benefit. At 64 mods, batch 4 saved only 0.58 ms paired warm and 1.23 ms
paired cold, with noisy cold results. One-mod results were noisy or slower.

The separate single-task diagnostic showed about 15.3–15.4% lower full-scope medians.
That mode removes too many cancellation checkpoints and is **not a production
candidate**. The bounded result justifies considering batch 4 for full composition,
but not claiming that the application is already faster. Before retaining it,
exercise ordered errors, mid-batch supersession/cancellation, cache-lease recovery
and native publication. Leave the one-mod path simple. No batching implementation
or extra dependency was added during this measurement slot.

### Onboarding roots and foreground priority

Five repeats per layout, 16 roots/1,024 mods. Balanced: 64 mods per root. Skewed:
904 mods in the first root, eight in each remaining root.

| Existing collector / layout | Serial width 1 | Current width 8 | Foreground permit wait at width 8 |
| --------------------------- | -------------: | --------------: | --------------------------------: |
| Balanced                    |      176.10 ms |        40.69 ms |                          19.79 ms |
| Skewed                      |      175.18 ms |       161.54 ms |                         176.40 ms |

Each row reports medians from independent probes; foreground wait need not equal
the census-only wall time. All foreground requests occurred while the background
scan was busy. Cancellation settlement medians were 0.207 ms balanced and 0.360 ms
skewed on this fixture; these do not bound cancellation during arbitrary slow I/O.
Output/scan counts and terminal-progress parity passed.

Width-1 permit spans expose root skew. Width-8 spans measure whole batches, **not
individual parallel roots**. The skewed layout's first batch occupied a median
157.09 ms of a 161.54 ms scan, so merely eliminating the boundary before the tiny
second batch cannot remove most of the cost. The large root itself and the permit
held across it need a targeted experiment. Do not introduce a new scheduler or
pause/resume state machine based on this result. A future comparison should first
reuse existing bounded branch parallelism and prove foreground/cancellation/output
semantics before changing production scheduling.

The log summarizer was corrected to recognize the first JSON event when Rust's
test-name prefix shares its line; all groups now include their five repeats.
This was an artifact parsing issue, not missing progress from the collector.

### Build warnings and remaining gate

The release library build passed with six `dead_code` warnings outside these
changes: `fail_interrupted_downloads`, `update_object_runtime_state_by_path`,
`list_terminal_batch_ids_for_staging_cleanup`, `list_active_staging_paths`,
`ReconcileModRow.size_bytes`, and `is_conflict_ignored`. The cleanup owner received
the evidence. This run does not claim a clean full Clippy/native certification.

### Native harness readiness

After the hotkey task's verified owned shutdown, the fixture profile now supports
release builds. Only the readonly profile receives `readonly-audit`; the private
identifier, fresh-build/hash, reset and driver ownership guards remain. Onboarding's
picker request resolves the executable from a validated debug/release mode.

The release contract went RED to GREEN; E2E typecheck, scoped ESLint and Prettier
passed. The nested fixture config selects five spec files (six performance cases
and nine coordinated preview/Dashboard cases). Both nested fixture and WPR configs
now use absolute spec paths, preventing config-relative paths from selecting no
tests. Their path/handshake contracts passed. This follows the installed WDIO 9
parser and [WDIO configuration rules](https://webdriver.io/docs/configuration).

The cleanup task handed off its three fixes and targeted checks before the final
source snapshot. A fresh fixture release then passed **15 cases in five spec files**
with no skipped cases and exit 0. Rust app build took 2m54s; WebView2 was
154.0.4258.62. All **1,577 native inputs remained unchanged** through shutdown.
Private executable SHA-256:
`c5a99c53b39903bbc9ea51e4bc7af7430108ec97cd1f1a96130f6c52aa00c3ca`.

Native evidence: `.audit-output/final-performance-audit-20261011/native-release/`.
The run covered real Windows folder selection for 6,000 primary and 80 background
mods, projection counts and indeterminate finishing, restart persistence, normal
and scoped sync, publication failure/retry, overlapping runtime supersession,
three additional process restarts, and nine preview/Dashboard cases.

Confirm-to-Home was **2,479.87 ms** on this owned fixture. Folder selection/operator
time is excluded; this is one observed release result, not a matched speedup or a
large real-library certification. The first game's visible classification advanced
through 1,513, 3,177, 4,886 and 6,000 folders, followed by mod-unit projection and
finishing without a percentage. Both actual picker paths were verified before
acknowledging them to the harness; an initial Computer Use click missed the modal
button, then screenshot-based selection succeeded. No picker was mocked.

The same first-game native log records preparation wall time 929 ms, projection
295 ms (apply wall 297 ms), and final handoff 397 ms. Projection includes about
78.5 ms of metadata work inside the mod phase and 9.2 ms of read-model work; these
are nested costs, not additional stages to sum. Its `classification_work_ms=4922`
is aggregate parallel work, **not 4.9 seconds of elapsed classification**. On the
subsequent 6,000-mod process restart, backend reconciliation took 810 ms total
(237 ms census, 375 ms classification, 196 ms projection). These observations
support investigating large-root traversal before adding another database cache.

Each of the three 301-mod reopen checks observed one watcher and one startup
baseline, no duplicate activation scan, background keybindings and deferred cleanup.
Their logged page times (291/319/326 ms) follow an explicit WebView navigation and
are **not process-start or cold-start timings**. Readonly startup/WPR remains the
separate measurement gate.

Owned app/driver/compiler processes were absent after teardown; scheduled fixture
cleanup completed. The installed database hash was unchanged. The app build passed
with five existing dead-code warnings: `update_object_runtime_state_by_path`,
`insert_mod_tx`, `ReconcileModRow.size_bytes`, `is_conflict_ignored` and
`classify_folder_strict`; this is not a clean Clippy claim.

After verified shutdown, the native/build slot was handed to the hotkey task for
the user's requested production build and installed-app update. Readonly startup
and WPR ran after that handoff completed and the installed app was closed; their
separate evidence and build identities are recorded below.

## Decision

Keep the current architecture. The evidence supports two small runtime-progress
corrections and clearer phase labels, followed by two bounded backend experiments.
It does not support another cache, scheduler service, unified indexing pipeline,
wholesale async traversal, or broad React memoization.

This is not a certification that all bottlenecks are gone. The final fixture result
certifies the frozen combined source and recorded private build above. It does not
certify subsequent source changes, the separate installed binary, or complete
smoothness. WPR attribution below applies to one measured renderer interval.

## Confirmed findings

### P2: normal-sync progress can move backward

`disk_snapshot.rs:644–692` increments the completed-root counter on Rayon workers
before calling progress. A worker holding count 7 can pause while count 8 is emitted.
The reporter's throttle lock ends before event emission (`orchestrator/request.rs:98–142`)
and does not enforce monotonic counts. `reconcileProgress.ts:16–25` forwards both
payloads, and `FolderGridSyncToast.tsx:65–76` renders their counts directly.

The actual hook replay reproduces 8 → 7. This is inaccurate progress, not evidence
of slower traversal, data corruption, or lost indexing.

Smallest correction: keep one previous accepted payload per listener. Reject a
lower count only when the previous nonterminal payload has the same run ID, phase
and total. Accept phase changes and different IDs. Clear the reference at terminal
events. This is constant-size state; no run history, ID parsing or new service.

### P3: disposed progress listener can still write state

`reconcileProgress.ts:16–29` waits for listener registration before unregistering,
without marking the subscription inactive synchronously. Tauri installs its JS
callback before the native registration promise resolves. An unmounted or switched
game's callback can therefore still call the store during that interval.

Replay confirms the write after unmount. Add an effect-owned active flag, clear it
at cleanup, and check it before accepting an event. Handle registration rejection
explicitly using the existing logging/error convention. No generic subscription
framework is warranted. Lasting damage or a missed disk repair was not demonstrated.

### Progress presentation differs between onboarding and normal sync

WelcomeScreen and GameIndexingOverlay already distinguish root scanning, projected
mod counts, and finishing without a measurable total. FolderGridSyncToast instead
shows a generic refresh label and any non-null total, including zero, without naming
the unit. A change from roots to mods can look like one overall percentage regressing.

Use the existing phase enum: discovery/census and finalization are indeterminate;
scanning shows completed folders, projection shows completed mods, with a positive
known total. Label the phase and unit. Preserve the short-operation display delay.
Do not invent a weighted overall percentage or advance a bar from elapsed time.
This follows [Microsoft's progress-control guidance](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/progress-controls).

### Low-priority ETA contract drift

`orchestrator/request.rs:98–154` calculates ScanningRoots ETA from time since the
whole reconcile started, including prior census, divided by classification roots.
That is not a phase-local estimate and also assumes similarly expensive roots.
No current production UI consumer displays `eta_ms`; this is not a visible countdown
bug or a speed optimization. Prefer null when next changing the reporter, without
adding an estimator or changing the public DTO solely for cleanup.

## Rejected findings and necessary safeguards

- Do not freeze a run permanently after Completed. Authority validation permits two
  passes total, one initial attempt and one retry, with the same reporter/run ID
  (`orchestrator/entry.rs:143–180`); each pass
  emits Completed. A new discovery event can legitimately follow it.
- An old terminal event clearing a newer run is reproducible by arbitrary replay,
  but production reachability was not established. Game and operation locks cover
  normal completion. IDs are allocated before lock acquisition and are not an
  execution-order contract. Do not parse their suffixes or add unbounded tombstones.
- Completed means that pass ended, not that projection was successfully applied.
  The coordinator already checks SourceUnavailable and other result statuses.
  No false-success defect was confirmed from the terminal event alone.
- Preserve activation, watcher generation, publication and supersession guards.
  They enforce different ownership boundaries; merging them is not simplification.
- Progress and `pending` currently share a store setter. Stale callbacks can affect
  both, but missing reconciliation has not been proved. Do not redesign dirty-state
  authority based solely on the replay.

## Original experiment criteria

| Candidate                              | Evidence now                                                                                                                                                                                 | Smallest experiment                                                                                                                                                                                      | Acceptance and risk controls                                                                                                                                                                                                          |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runtime harvest dispatch per mod       | Full/scoped composition awaits one `spawn_blocking` per mod in `post_apply.rs:1031–1090,1160`; historical invalidation + harvest was about 804 ms for 2,048 mods. Dispatch share is unknown. | Existing cache/harvest fixture: current serial versus bounded batch widths 1/2/4, including dispatch and worker overhead. Include full, scoped and one-mod controls.                                     | Alternating paired runs; retain only improvement exceeding run noise. Exact bytes/order/cache results; error behavior, supersession checkpoints and post-cancel work remain bounded. File currently owned by hotkey task.             |
| Onboarding root skew / foreground wait | `disk_snapshot.rs:889–934` joins root batches while holding ScanPermit; foreground priority in `onboarding_session.rs:129` waits for release. Long roots can dominate. No new timing yet.    | Extend existing onboarding parallelism benchmark with one-large/many-small roots and a foreground switch during background work. Record per-root wall, permit wait, total wall and cancellation latency. | Compare existing batching with existing bounded Rayon facilities only if skew/wait is material. Preserve deterministic error/progress/output, foreground ownership and joined shutdown. Do not copy normal census scheduling blindly. |

Use existing benchmark entry points `benchmark_actual_cache_batch_invalidation` and
`benchmark_onboarding_session_parallelism`. Lock the source/build hash before running,
then use a unique Cargo test filter with `--ignored --nocapture --test-threads=1`.
Use a full module name if adding `--exact`. Run Cargo only after the coordinated
compiler slot is released; no parallel native/build timing contamination.

Warm harvest still verifies INI contents. Replacing hashes with mtime is not an
equivalent optimization. [Tokio documents that started blocking tasks cannot be
aborted](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html), so batching
must retain bounded cancellation checkpoints. Use the existing
[Rayon pool](https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html), not a custom scheduler.

## Existing measurements, not new speedup claims

| Area                        | Retained evidence                                                                     | Scope                                                                                                 |
| --------------------------- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Bootstrap                   | Initial static JS 1,838,606 → 1,423,433 bytes, −22.6%; eager Recharts modules 195 → 0 | Matched bundle experiment; no matched native startup gain established.                                |
| Normal census               | 900.899 → 402.092 ms, −55.37%                                                         | Seven paired release stage runs; 30,290 entries and all 149 progress events equal.                    |
| Normal classification reuse | 214.715 → 205.321 ms, −4.38%                                                          | Read-only GIMI stage benchmark; warm/uncontrolled OS cache, not app startup.                          |
| First-index classification  | 603.623 → 279.135 ms, −53.76%                                                         | Controlled single-root branch-budget fixture.                                                         |
| Finish Setup                | 7,359 → 3,920 ms observed                                                             | Two debug native observations with concurrent source evolution; not a causal release comparison.      |
| Database read-model         | About 2.5 ms in latest fixture, 7–8 ms on 4,243-mod library                           | Too small to justify another cache. Earlier unchanged-projection guard experiment was neutral/slower. |

Details: [scan/census/UI evidence](scan-reuse-census-ui-20261011.md),
[first-index evidence](indexing-efficiency-20261010.md),
[runtime evidence](runtime-indexing-execution-20261010.md),
[bootstrap/native evidence](bootstrap-bundle-wpr-20261011.md).

Keep publication-lock migration traversal: roughly 389 ms all-active versus 80 ms
mostly-disabled after pruning. Moving it outside the lock lacks the necessary
authority proof. Keep catalog/avatar integrity checks; prior byte-reuse timings
overlapped and do not justify another cache. No whole-pipeline rewrite is indicated.

## Initial audit verification, before approved execution

- Existing WelcomeScreen, GameIndexingOverlay and FolderGridSyncToast suites:
  **20 PASS**. Existing runtime progress tests: **2 PASS**.
- Isolated actual-hook replay: **4 RED**, of which counter regression and callback
  after unmount are confirmed findings. The two terminal-order replays remain
  defensive/unproven. Added valid same-ID retry control: **1 PASS**.
- The first isolated control config missed production demo-disable aliases and
  caused 9 harness failures. Reusing the application Vite config restored all 20
  controls; these failures are not application regressions. Existing mock-motion
  and act warnings remain in the control tests.
- Five audited progress/backend source hashes remained unchanged through replay.
  No Rust compiler, application build, native test, installed-config change or
  real-GIMI operation was performed in that initial audit.
- Artifacts: `.audit-output/final-performance-audit-20261011/`, especially
  `progress-replay-production-aliases.json`, `progress-control-production-aliases.json`
  and `audited-source-hashes.json`. These are unit replays, not native E2E.

## Readonly startup and WPR attribution

The previous fresh readonly release passed four native process launches with
148 objects/4,243 mods, one watcher/baseline, identical fixture manifests, restored
private configuration and no source drift. It did not exercise the Finish Setup
picker or runtime publication writes. It also observed a maximum 146.7 ms frame
gap and 236 ms buffered long task. Those results do not certify perfect smoothness.

Early sampled frame gaps contained mostly V8 idle time. Attribute host scheduling,
GPU/compositor and early bootstrap with the prepared WebView2 CPU + GPU/DWM WPR
capture before changing React or animations. This follows [Microsoft's WebView2 tracing and initial
payload guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance).
The Administrator capture on October 11 completed, but its 4,776,263,680-byte ETL
lost **83,249,176 events**. It is unsuitable for frame-gap attribution or startup
performance claims. `xperf -tle -a tracestats -detail` was used only to diagnose
capture volume: retained events included 65,174,850 syscall enter/exit events and
8,062,970 CPU idle enter/exit events. The verbose built-in composition profile
enabled syscall/all-fault streams; the Microsoft CPU profile enabled idle states.
The failed trace and provider statistics remain in the `wpr-native/` artifacts.

The first native launch completed, then a restart hit an invalid IPC Origin. The
readiness predicate accepted an injected Tauri bridge in an initial blank document.
A direct callback regression reproduced that acceptance; the guard now also
requires the local Tauri origin. Completed measurements are saved after each run
so a later failure preserves the earlier frame timestamps. This is a harness fix,
not a demonstrated application startup improvement.

The focused retry profile retains sampled CPU, scheduling stacks, disk/hard-fault
events and the stable WebView2 provider masks, combined with GPU and composition
**light** profiles. Profile validation confirms the heavy syscall/all-fault/idle
streams are absent. The stop acknowledgement allows a bounded ten-minute merge
(the failed capture exceeded the previous two-minute limit); timeout/failure still
restores private configuration. First-attempt After/Verify manifests are identical,
private configuration is restored, installed executable/database hashes match,
and all owned native processes have exited.

The corrected untraced release run passed all four process launches with 148
objects/4,243 mods. Empty-index core observation was 3,913 ms; three indexed
restarts observed core ready in 1,140–1,169 ms, shell in 711–792 ms and optional
Home completion in 1,783–2,826 ms. These include driver/polling overhead and have
uncontrolled OS caches. The maximum observed frame gap was 133.4 ms, so this is
not a smoothness certification. Build SHA-256:
`4285bf25bb6887644ba401e387b36adb2ab6ecf0afdf22c11fb5e8e57b7f7974`;
1,578 inputs were unchanged and Before/After/Verify fixture manifests matched.
Evidence: `native-readiness-check/`.

The first focused WPR retry also passed all four launches, but its 1,526,726,656-byte
ETL still lost **682,284 events**. Its timings are not a performance baseline.
WPA exporter correctly refused attribution from the lossy trace. Filtered `xperf`
diagnostics showed ETW allocation failures in the event collector for unrelated
`msedge.exe` Edge and GPU events, rather than proving an EMMM/React defect.
The successful retry profile restricts browser-provider events with
[`ProcessExeFilter`](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/eventprovider)
to `msedgewebview2.exe` and increases the custom event-buffer allocation from
64 to 256 MiB (424 MiB after combining built-in profiles). System CPU/GPU/DWM
visibility remains available.

For attribution, align the native probe's time origin with the trace and inspect
the affected host/renderer threads during the observed gaps. Use sampled CPU for
executing work, precise CPU scheduling for Ready versus Waiting time, and DWM/GPU
events for presentation. Idle or Waiting time alone does not establish a defect;
it matters when it delays the operation being measured. This follows Microsoft's
[CPU analysis guide](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/cpu-analysis).

### Completed zero-loss capture

`wpr-native/retry-2/webview2-startup.etl` is 1,157,627,904 bytes and covers
26.565697 seconds from 2026-10-11 14:35:16.571605 UTC. The trace header reports
**0 lost events and 0 lost buffers**. All four native process launches passed;
1,578 captured inputs had zero drift. Tested executable SHA-256:
`a1e7062284018534d77d189d58a93638d36c0dc5f921f8ee4332afe89b6aa603`.
The executable is preserved as `retry-2/emmm-readonly-audit.tested.exe`, independent
of subsequent shared-target builds. WPA exports completed successfully.

The run-2 probe observed a 133.2 ms frame interval, mapped to ETW relative time
20.773995–20.907195 s using its page time origin. Navigation and a matching
131.224 ms V8 callback establish the UI renderer as PID 83256, main thread 53340;
the other renderer process is not substituted for this page.

Integrating raw context-switch and ReadyThread events, clipped to that interval:

| UI renderer state |   Duration |
| ----------------- | ---------: |
| Running           | 119.167 ms |
| Ready             |   6.394 ms |
| DeferredReady     |   0.015 ms |
| Waiting           |   7.624 ms |

These states cover the full 133.2 ms window. Running includes native/kernel and
instrumentation work; it does not mean 119 ms of application JavaScript. V8
callbacks nevertheless overlap the gap: one spans 40.816 ms, another contributes
24.684 ms of overlap, with shorter callbacks and module evaluation also present.
Nested callback spans must not be summed. Before the gap, a 131.224 ms V8 callback
matches the native probe's 131 ms long task; module evaluation also spans 110.434 ms.

The native main thread ran for 15.474 ms and spent only 1.283 ms Ready in the same
interval. This sample supports investigating renderer/bootstrap work before CPU
scheduler changes. GPU/DWM exports provide limited corroboration, not comprehensive
exclusion of presentation delays. Only 40 sampled CPU events were available across
the trace, so sampled stacks cannot identify application functions. Precise CPU
analysis uses the separate 3,862,796 context switches and 1,231,230 ReadyThread
events. A collapsed sampled-CPU idle percentage is not evidence that this renderer
was idle during the gap.

Next useful experiment: align a native WebView2 JavaScript CPU profile with these
callbacks and identify the responsible module/function before deferring work or
changing React renders. This capture alone does not justify broad memoization,
another worker service, a scheduler rewrite or blaming a particular component.
WPR visibly perturbed startup; use the untraced baseline above for observed startup
times, and do not claim an improvement from these capture runs.

Evidence under `wpr-native/retry-2/`: `trace-header.txt`, `native-verification.json`,
`native-startup-measurements.json`, `frame-thread-states.json`,
`webview-frame-spans.json`, `frame-gap-tables/` and `wpa-tables/`.
The two local analysis scripts preserve the selected time window and reject
incomplete/out-of-order scheduling coverage.

Before/After/Verify fixture manifests all match the digest above. Private config
was restored with zero real-library bindings; installed executable/database hashes
were unchanged and owned native processes had exited. WPR recording stopped.
This capture used the protected 4,243-mod fixture, not the real GIMI library.
No reboot or storage-cold measurement occurred; desktop load remained uncontrolled.
The game was absent and Edge remained open. New source changes after the handoff
require their own fresh build/native validation.

The new hotkey feature introduces another HTML entry. Its owner accepted the
bootstrap-guard follow-up: select main `index.html` explicitly rather than the
first `isEntry` chunk, keep manualChunks removed, and create the notice WebView
only after an opted-in event. Recheck the initial graph on that final source.

## Practical order

1. Complete: small progress guards and phase/unit presentation with regression
   tests, including the legitimate same-ID retry control.
2. Complete: release harvest and onboarding foreground/skew measurements. Batch 4
   remains a tested prototype; production cancellation/error semantics need a
   separately scoped follow-up before retaining it.
3. Complete: 15 native release fixture cases on the frozen combined build.
4. Complete: corrected readonly startup passed four launches without tracing;
   focused WPR also passed four launches with zero lost events/buffers. One frame
   gap is attributed to renderer execution; exact JavaScript function attribution
   remains a bounded follow-up, not a proven React defect.

The separately coordinated Dashboard/preview task owns repeated/out-of-order
external health analysis; do not duplicate that implementation here.

## Skills and documentation fit

The explicitly requested `find-skills` discovery was combined with performance
optimization, systematic debugging, architecture review, targeted React guidance,
Karpathy's small-change guidelines and the repository's native E2E skill. Existing
skills are sufficient; no package or skill was installed. Next.js-only prescriptions
were not applied to Tauri. Stable subscriptions/snapshots follow
[React's external-store guidance](https://react.dev/reference/react/useSyncExternalStore).

Low-impact instruction drift remains: `.agent/rules/code_standards.md` mentions old
`src-tauri/commands/services/database` and `lib/bindings.ts` locations, while current
code uses modules and `shared/api/tauri/bindings.ts`. Correct those paths during the
next rules cleanup; this is navigation drift, not an indexing bottleneck.
