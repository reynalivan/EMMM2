# Follow-up startup performance experiments

Implement the user's approved small experiments after the real release audit.
No persistent cache, dependency, service or filesystem mutation is authorized.

## Scope and gates

1. Reuse the normal classifier's strict `FolderScan` for container traversal,
   as onboarding already does. Preserve projection fields/order, directory link
   and hidden-folder policy, missing-path `SourceUnavailable` and strict INI errors.
2. Profile census per root and compare current four-root batches with a test-only
   four-thread continuous Rayon run. Retain a production scheduling change only
   if benefit exceeds run variance and online progress/error prefix stays correct
   without a custom scheduler. Pool creation must be included in measurements.
3. Capture CPU samples from the native release WebView2 session on an owned
   4,243-mod / 640-keybinding fixture. Attribute long tasks before changing UI.
   Profiling runs include overhead and are separate from final uninstrumented runs.

Use alternating paired measurements, exact output parity and existing concurrency
regressions. A neutral experiment is discarded and documented. Read-only GIMI
stage benchmarks do not configure an application or certify end-to-end startup.

## Verification

- Existing normal sync and new container/error/link parity cases before/after.
- Optimized stage benchmarks, same fixture/root and parallelism in each pair.
- Strict Clippy, scoped Rust formatting, E2E typecheck/lint.
- Fresh native release build; private identifier/IPC/interlock/config restoration.
- Full owned-fixture Before/After checksum and final metadata/structure Verify.
- Record retained/rejected experiments, measurement conditions and residual limits.

## Execution

Evidence is retained under
`.audit-output/startup-next-20261011/`; user library files are never repaired,
renamed or removed. Native ports/build/GUI slots are coordinated with the existing
hotkey validation chat under prior explicit messaging authorization.

### Classification acceptance

Baseline and post-edit normal tests passed 14 cases; manual benchmarks stayed
ignored until explicitly run. Three new cases cover nested containers/strict
errors, missing/file paths and Windows directory links (executed without the
permission skip). The optimized release benchmark passed both datasets with
seven alternating pairs and exact projection fields/order/counts.

| Dataset                                       | Original median | Reuse median | Reduction | Positive pairs |
| --------------------------------------------- | --------------: | -----------: | --------: | -------------: |
| Container-rich owned fixture, 1,024 mods      |      174.713 ms |   140.968 ms |    19.31% |            7/7 |
| Read-only GIMI, 4,344 raw terminal candidates |      214.715 ms |   205.321 ms |     4.38% |            6/7 |

GIMI paired median saving was 9.394 ms, paired MAD 1.909 ms. Retain the small
reuse change with a modest classification claim. Raw projection candidates and
the previous persisted UI count of 4,243 mods are different measurement scopes.
Cache was warmed/uncontrolled; these results do not establish storage-cold or
end-to-end startup improvement. Compilation took 10m34s and is outside timings.

### Census scheduling ceiling

Three rounds of 148-root profiling found consistent batch skew: cumulative batch
work 945–985 ms versus summed root work divided by four of 349–361 ms. Seven
alternating complete census pairs passed exact all-field/order parity:
954.556 ms median for current batches versus 411.384 ms for a test-only bounded
pool, including creation/drop (56.90%; 7/7 faster, paired saving 550.298 ms,
MAD 15.174 ms). This pool has no online progress and is not the shipped candidate.

Proceed with a standard four-worker Rayon pool and completion channel only after
deterministic tests establish bounded reads, slow-root bypass, ordered progress
and earliest indexed errors. Normal passes hold the shared OperationLock and
background game loops are serial; the limit is per pass, not a universal process
cap. Avoid persistent pool caches or another scheduler/service.

### Production census acceptance

The controlled slow-root test failed on the original batching implementation;
the bounded scheduler then passed all 12 targeted cases. The broader reconciliation
suite passed 287 tests (15 manual/benchmark cases ignored), and focused concurrency
review found no actionable issue. Scoped Rayon workers join before return; the
standard channel stops queued jobs when the ordered consumer returns its error.

Seven alternating release pairs compare the actual production collector with
frozen original four-root batches, including progress callbacks, sorting and
worker startup/joined shutdown. All 30,290 entries (four fields/order) and all
149 progress events (six fields/order) matched exactly. Median time fell from
900.899 ms to 402.092 ms (55.37%); all seven pairs improved. Retain this scheduler
change. The earlier 411.384 ms ceiling remains a separate experiment; neither
result establishes full application startup timing or storage-cold performance.

### UI attribution and final verification

Native CPU sampling and renderer metrics were collected through the existing
Tauri/WebView2 driver; no separate browser or dependency was added. Profiling is
opt-in and starts after IPC attachment. Failure-path contract checks cover
partial setup cleanup, collection after an intervening assertion, idempotent stop
and preservation of the primary error.

The first run exposed a measurement defect: WDIO sets `NODE_ENV=test` before its
build hook. Vite's production mode does not override that value, so the optimized
Rust executable embedded React development (`jsxDEV` observed in CPU samples).
The ordinary application build correctly produces React production. The harness
now sets `NODE_ENV=production` only for its release build child, preserving all
other environment values and its audit guards. Core bundle size is 178,695 bytes
instead of approximately 362.76 kB; ordinary production assets contain no `jsxDEV` calls.

The corrected production profile passed the empty-index launch and three true
process restarts: 148 objects, 4,243 mods, one watcher and one StartupBoot baseline
on each launch. All four observation windows recorded zero long tasks; maximum
frame gap was 33.5 ms. Renderer script duration was 76.7–160.2 ms, layout
5.6–30.8 ms and style 29.0–59.3 ms across the profiler/metrics interval. Sampled
chart self time was 9.9–23.3 ms. These results do not justify new UI memoization,
animation changes, caches or a React profiling build. No production UI code was
changed.

The development-mode first run is retained as diagnostic evidence, not a
production baseline or an application speedup claim. Source/backend corrections
in the coordinated hotkey task also separate the two builds; this is not an
alternating paired UI optimization experiment. Prior GIMI release UI observations
used the same development core bundle and are qualified in their report.

Production profile evidence: `ui-production-profile-p3/`, executable SHA256
`45aef858f0fb33092440ef7096b8b9dad890d44e4b62c45a87150aa8a1b0021f`, 1,524 selected
inputs unchanged. Full fixture Before/After/Verify matched exactly and private
configuration was restored with zero library bindings.

Two ordinary release runs subsequently passed all readiness/count/Home-error
checks, full fixture integrity and configuration restoration. The first observed
one 63 ms long task and 100 ms frame gaps. Moving the count-validation IPC outside
the measured UI interval did not remove gaps: the second run observed 53/61 ms
tasks and gaps up to 150.1 ms. Do not claim perfect smoothness or blame that extra
query. The validation remains outside timing because it is not application work.

The probe now records event timestamps, buffers earlier long tasks, drains queued
entries at stop and resets the frame baseline on visibility changes. CPU capture
starts before bridge verification; a failed ancillary metrics read retains the
CPU artifact. Focused failure/state contracts passed, including regression
checks that failed before the fixes. Review found no remaining material issue.

The early profile passed four launches with unchanged fixture/configuration and
1,524 selected inputs. Its sampled restart frame gaps of 69.3/71.2 ms contained
approximately 56.2/51.2 ms of V8 idle samples. This does not prove a React or chart
bottleneck. Buffered bootstrap tasks of 54–122 ms all ended before CPU sampling
began; even the earlier attachment cannot attribute them. Standard WebView2
startup tracing was tested through a process-local environment override. The
four-launch native spec passed, but WebView2 produced no requested trace file;
this is not a successful launch trace. No persistent application or system
configuration was changed. WPR/WPA are installed, but the current session is not
Administrator. Earliest bootstrap and compositor/host scheduling attribution
therefore remains open pending an operator-assisted Windows trace. No React
refactor or animation removal is justified by the current evidence.

Ordinary observation run (before the subsequent coordinated watcher scope fix):

| Launch                     | Home observed | Core observed | Optional Home settled | Backend reconcile |
| -------------------------- | ------------: | ------------: | --------------------: | ----------------: |
| Empty isolated index       |       2.172 s |       2.336 s |               2.879 s |            513 ms |
| Restart median (3 samples) |       0.838 s |       1.121 s |               1.737 s |            445 ms |

These are fixture observations with harness/readiness polling overhead and
uncontrolled cache/load. They are neither exact first paint nor real GIMI
production-frontend certification. Early CPU and startup-flag runs remain
diagnostics and must not replace these ordinary measurements.

All six native runs in this follow-up passed the selected spec (four process
launches each), source/binary checks, full fixture Before/After/Verify and private
configuration restoration. Evidence directories: `ui-profile-release-p3`
(development frontend), `ui-production-profile-p3`, `ui-production-final-p3`,
`ui-production-observation-p3`, `ui-early-profile-p3` and `ui-startup-trace-p3`.
The protected fixture retained 4,248 files, 4,395 directories including root,
242,611 bytes, no links and digest
`59F14172DAA27661C6BC2D012054D580BF85925827FEC9A21FB2A19C66B9A72B`.
No real GIMI root was bound or mutated in this UI phase.

Coordinated changes belong to their respective chats: hotkey replay/alias fixes
and a narrowly scoped watcher fix for unrelated runtime-parent temporary files;
Randomizer dialog scrolling and two final functional native cases. Their checks
do not certify UI frame performance. The last startup-flag run includes those
source changes and passed integrity/identity checks; installed application update
and actual-game validation are outside this task's scope.

Unrelated cleanup source changes triggered an additional 10m13s release compile
before the census experiment. The user authorized coordination with that chat;
its Rust/frontend source is now frozen while this task completes its measurements.
No competing native/compiler run is included in the paired timing windows.

Strict all-target Clippy with `readonly-audit` and `-D warnings` passed after the
cleanup owner removed an unrelated trailing return in its archive guard. E2E
typecheck/lint, scoped Rust/Markdown formatting and diff checks passed. Post-benchmark
GIMI metadata/structure verification matched the previous complete checksum manifest:
375,100 files, 30,318 directories including root, 632,025,086,331 bytes and zero
links, digest `F88F86A5AD7EBE814ADBFC224328FC294510E18EC22A8C71CBF03D66A976C922`.
This round did not recompute all asset-content hashes.

The user confirmed the desktop was unlocked for UI profiling on October 11.
The previously agreed hotkey validation owns the next safe native/compiler slot;
capture release inputs after its fixes and the cleanup owner's scoped dialog fix
settle. No UI speed claim or speculative UI optimization is accepted yet.

Use the owned 148-root fixture with 4,243 mods and 640 keybindings. Its repeated
Before checksum remained identical (`59F14172...B9A72B`); its small asset payload
models UI result volume, not hundreds of GB of storage throughput. Native CPU
sampling and renderer metrics start after IPC attachment. Correlate script,
layout/style, sampled functions/chunks, long tasks and visible frame gaps before
choosing a production change. Task duration includes work beyond script/layout
and must not be treated as their sum.

Keep instrumented runs separate from ordinary release measurements. Validate
the live CDP route through the owned Tauri/WebView2 driver before relying on the
profile; helper contracts alone cannot establish native support. Record one
empty-index process start and three indexed process restarts without a page
remount. If a scoped change is justified, compare matched conditions and discard
gains within measurement noise; three restarts do not establish p95. Check full
fixture After/Verify, source/binary identity and isolated configuration restoration
after the run. OS cache remains uncontrolled and runtime publication is excluded.

The coordinated fresh private debug native run passed the three
`normal-sync-keyviewer.e2e.ts` cases and the one `startup-reopen.e2e.ts` case.
The combined run also passed three import cases and two of six unrelated audit
cases; four audit cases failed and remain with that task's owner. Do not treat
the combined 9-pass/4-fail result as an all-app pass. This is functional native
WebView2/IPC/disk coverage, not final release frame-performance certification.

Native functional evidence: `.audit-output/audit-followup-20261011/native.log`,
1,422 selected inputs unchanged through that first run, private debug executable
SHA256 `4aab54336a46698eef80894cc7acfd5a5882c84f7192dc5e5ae9a349ccfc7aed`.
Owned teardown was confirmed. Subsequent unrelated frontend/test fixes belong to
that task's next audit-only run; they do not retroactively certify its failing cases.
