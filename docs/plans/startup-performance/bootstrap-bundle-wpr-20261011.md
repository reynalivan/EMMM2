# Bootstrap bundle and native WebView2 trace

The user approved both experiments. Keep real GIMI files untouched and use the
owned UI fixture/private read-only audit identifier for native measurements.

## Bundle evidence and candidate

The manual callback assigns React to `vendor-core`, but recursive dependency
capture places React's CommonJS runtime in `vendor-charts`. Core then imports
charts. Prioritized explicit core grouping fixes that edge but leaves all 195
Recharts modules in the initial static graph: no useful payload reduction.

Private automatic-splitting build removes those eager chart modules while
retaining Dashboard charts. Initial JS is 1,423,433 rather than 1,838,606 bytes
(-22.6%); summed gzip is 417,458 rather than 535,110 bytes. These are bundle
measurements, not startup speed claims. A second baseline/automatic comparison
in `matched-inputs` reproduced these exact values with identical selected build
input hashes before and after. Native timing and route validation remain open.

The implemented change removes only the 34-line manual grouping configuration
from `vite.config.ts`. No new lazy state, dependency, scheduler, cache or component
rewrite. The write-free build guard failed before the change (195 eager Recharts
modules) and now passes (three initial static chunks, charts retained lazily).
Application/config TypeScript, scoped zero-warning lint and formatter checks pass.
Focused review found no actionable issue; the fresh release native startup spec
passed all four process launches.

Validate native onboarding and Home on a fresh release, retaining all guards and
the app's actual watcher/recovery flow. Compare matched conditions; do not infer
p95 or storage-cold performance from a few process restarts. This run validates
Home and an initially empty isolated index; it does not exercise Finish Setup or
the Dashboard chart interaction in the native UI.

## Final native fixture validation

Evidence: `.audit-output/bootstrap-bundle-20261011/native-final/`. The ordinary
production frontend is embedded in the private read-only audit release; CPU/WPR
profiling was off. The harness built current source before running the spec.

| Launch               | Home wall (ms) | Core wall (ms) | Optional Home wall (ms) | Backend reconcile (ms) |
| -------------------- | -------------: | -------------: | ----------------------: | ---------------------: |
| Empty isolated index |          6,155 |          6,214 |                   6,647 |                    706 |
| Indexed restart 1    |          2,202 |          2,365 |                   3,207 |                    478 |
| Indexed restart 2    |          1,437 |          1,606 |                   2,249 |                    539 |
| Indexed restart 3    |            845 |          1,325 |                   2,070 |                    545 |

All launches retained 148 objects/4,243 mods, one watcher registration and one
StartupBoot reconcile, with no duplicate GameSwitched reconcile or Home error.
The mutation-denial check passed. Pages stayed visible at 1,280×800 and DPR1.25;
GenshinImpact remained running. Wall times include native driver/session/polling
overhead. OS caches are uncontrolled; these are descriptive values, not a matched
before/after speed comparison. Frame gaps still reach 146.7 ms and the buffered
initial long task reaches 236 ms; their cause remains unassigned pending WPR.

Full fixture Before/After checksums and Verify metadata match:
`59F14172DAA27661C6BC2D012054D580BF85925827FEC9A21FB2A19C66B9A72B`.
Config restoration reports zero audited-library bindings. Selected 1,529 build
inputs have no drift; private executable SHA256 is
`b90c1df9974bd35f5d4a3aee998825c23ac60c25115b1ebaf80d7021b1600a8a`.
Owned driver/native processes stopped. The installed executable still matches
the earlier chat's hash. Real GIMI was never bound or read by this run.

The release build retains five pre-existing unused Rust warnings; this config
change does not fix or certify the release-profile Clippy surface.

## WPR capture

Microsoft's current guide recommends `WebView2_CPU.wprp` for CPU/scheduler data;
`WebView2.wprp` is also downloaded for the explicitly requested smaller profile.
Both official XML profiles validate with the installed WPR. WPR/WPA are already
installed, but the agent session is not Administrator.

Use an operator's Administrator terminal; no automatic elevation, registry
changes, cancellation of another recording or upload. Check `wpr -status` first.
Start only once the private harness build is complete and the start gate exists:

```powershell
wpr -start "E:\code\projects\EMMM2\.audit-output\bootstrap-bundle-20261011\wpr\WebView2_CPU.wprp" -filemode
```

After the owned native processes stop, use the run's output directory:

```powershell
wpr -stop "<owned-run-output>\webview2-startup.etl" "EMMM isolated WebView2 startup"
```

The temporary WPR config wraps launcher `onPrepare`/`onComplete` with start/stop
operator acknowledgements. These share launcher state; worker hooks retain the
existing session timing and driver lifecycle. Request/ack filenames contain the
run token, so retries cannot consume an earlier run's acknowledgement. Stop is
requested after worker shutdown and before terminal config restoration; a stop
failure still runs restoration. Preserve source/binary identity and full fixture
Before/After/Verify. Wall timing excludes operator waiting and compilation. A
timeout is a failed capture, and the operator must stop any recording they started.
The focused adapter contract failed before the corrections and passed afterward;
TypeScript and read-only review passed. Privileged recording remains untested.

The official CPU profile includes sampled CPU and thread scheduling, but no
dedicated Windows GPU/DWM providers. Built-in `GPU` and `DesktopComposition`
profiles are available and their combined XML was exported locally. Add these
to the same short capture when compositor/GPU attribution is required:

```powershell
wpr -start "E:\code\projects\EMMM2\.audit-output\bootstrap-bundle-20261011\wpr\WebView2_CPU.wprp" -start GPU -start DesktopComposition -filemode
```

This combination has not yet been tested in a privileged recording. Do not
claim GPU attribution from the CPU profile alone. Keep trace duration short and
separate instrumented attribution from ordinary release timing.

Analyze the owned app and WebView2 process tree in WPA: sampled CPU, ready/wait
time, context switches, disk/hard faults, process creation/navigation and relevant
rendering events. Match startup markers and frame/task timestamps; distinguish
JavaScript work from compositor/host scheduling. A trace file or provider list
alone does not establish successful attribution. Validate any retained fix on a
normal release without profiling afterward.

Sources: [Microsoft ETW guide](https://github.com/MicrosoftEdge/WebView2Feedback/blob/main/diagnostics/etw.md),
[Rolldown recursive capture](https://rolldown.rs/in-depth/manual-code-splitting),
[WPR multi-profile recording](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/wpr-command-line-options),
[WDIO process-separated hooks](https://webdriver.io/docs/configuration).

## Coordination

The original hotkey chat completed Fullscreen native checks and its normal
installed-update/hash verification, then released source/native ownership.
Parent applied the Vite deletion afterward and took the build/native slot for
the private read-only fixture validation. Other Settings/theme changes remain
with their owner. Initial bundle experiments used private outputs. The final
native harness builds shared `dist` and a private audit binary; it neither
updates nor launches the installed app and never binds real GIMI.
