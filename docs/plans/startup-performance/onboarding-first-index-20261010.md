# Onboarding and first-index corrections

User approved all audit findings on 2026-10-10. No production library benchmark
or production configuration changes are part of this work.

## Behavior

- Unrecoverable activation Failed rejects immediately. SourceUnavailable retains
  the existing bounded automatic recovery; Ready still requires a disk revision.
- Scoped foreground/application/activation owners protect live sessions from
  idle TTL. Expired idle sessions cannot regain ownership; explicit cancellation
  interrupts directory traversal and priority waits. Idle expiry restarts after
  the last owner releases.
- Preparation counters are monotonic and serialized; actual traversal reports
  classified folders within a root with throttling. Unknown totals stay unknown.
- Progress registration is awaited before its work starts. Preparation binds
  the known session/game; disk events also require the reason and exact attempt.
  A legitimate recheck has a higher attempt suffix and fresh reporter. Old
  callbacks and terminal events cannot overwrite retry/recheck progress. Failed
  registration is retried rather than caching a permanent rejected promise.
- Raw watcher installation and fused discovery share one blocking worker with
  watcher-before-scan ordering. Logs separate preparation/apply/handoff wall
  time from summed census/classification work.
- Event DTOs moved unchanged into a pure type module, preserving facade exports
  while allowing NodeNext native test imports. No IPC field or schema change.

## Comparative measurement

Actual onboarding session/raw watcher/fused discovery plus direct precomputed DB
projection on 1,024 owned temporary mods and 16 roots. Five alternating repeats
at widths 1/2/4/8; all four census fields, mod paths and DB counts match.

| Distribution                     |   Width 1 |   Width 2 |   Width 4 |   Width 8 |
| -------------------------------- | --------: | --------: | --------: | --------: |
| Balanced, preparation / total ms | 219 / 418 | 120 / 302 |  79 / 291 |  52 / 251 |
| Skewed, preparation / total ms   | 223 / 406 | 209 / 396 | 193 / 376 | 194 / 378 |

Retain existing width **8**. Balanced total is 13.75% below width 4; skewed is
effectively unchanged. No new pool, persistent cache or higher concurrency.

The skewed fixture places 896 mods in one root. OS cache is uncontrolled; newly
created fixtures are reference-read before trials. Integer stage times truncate
independently. These totals exclude adapter journal validation, managed watcher
handoff, KeyViewer, Home and process startup. They are not real-library cold
startup or p95 measurements. Forty raw rows and context:
`.audit-output/onboarding-first-index-20261010/session-parallelism.json`.

## Verification ledger

- Backend targeted onboarding: 37 passed / 2 ignored. RED regressions cover
  ordinal/phase progress, foreground TTL, expired-orphan resurrection and
  intra-root activity/cancellation; existing activation/parity cases retained.
- Related frontend: 58 passed across 9 files, including readiness failure,
  listener retry/barriers, exact event ownership and recheck attempts.
- Final full Rust library suite: 1,674 passed / zero failed / 22 ignored,
  83.31 seconds. An earlier parallel run failed a new Restart Manager holder
  test outside onboarding; isolated rerun and final full rerun pass.
- Strict all-target Clippy, frontend/E2E typechecks, scoped ESLint, full Rust
  formatting, scoped Prettier and whitespace checks pass. Clippy stabilization
  retrieves the existing managed pool through AppHandle in two command wrappers
  and removes an obsolete error wrapper; JavaScript payloads remain unchanged.
- Native startup spec passes three 301-mod process restarts on a fresh private
  build, one watcher/baseline each, no duplicate GameSwitched full baseline.

- Native first-index picker run: **2 passed** on the freshly built private native
  executable, session `f0d6e1e1235574683754948fefdd285b`. Both real Windows
  dialogs selected verified owned temporary fixtures. The actual Confirm flow
  indexed 6,000 mods in one large root, displayed intra-root classified progress,
  reached Home in 7,551 ms, and indexed another 80 mods in background. A native
  process restart retained the first index and active game. Twenty-one snapshot
  events and disk/UI observations are saved in
  `.audit-output/onboarding-first-index-20261010/native-picker/native-progress.json`.
  Both scheduled fixtures were removed only after the owned process tree stopped.
  This single synthetic run uses a debug native build and warm/uncontrolled OS
  cache; its Home measurement excludes manual folder selection. It does not
  certify cold startup, real-library throughput, p95 or release-build latency.

Earlier attempts timed out when the QA window was manually used
for another folder, as confirmed by the user. Those failures do not certify the
fixture flow. The QA title now clearly identifies synthetic fixtures. Its prior
profile was archived intact before clean isolation; production profile untouched.
An initial build was stopped because an empty Windows environment override did
not suppress the configured collector. The harness now supplies whitespace that
trims to no collector before every private build. No operator clicked Confirm
or selected the user's folder during these attempts.

The archived private profile remains intact at
`C:\Users\yusri\AppData\Roaming\com.reynalivan.emmm.e2e.archive-36439c34-d6da-49ea-af09-c7c70d164ab6`.
One uncaptured temporary fixture from an earlier aborted run is retained because
its cleanup ownership was not recorded; it was not removed based on its name.
The final private profile has synthetic fixture bindings only. Real GIMI files
were not used or modified by this test automation.
Read-only SQLite verification confirms both games' required paths and every
non-null optional launcher/game path stay within the two owned fixture roots;
both roots are gone and the archived profile still exists. Isolation evidence:
`.audit-output/onboarding-first-index-20261010/native-isolation-final.json`.
Tested private binary SHA-256:
`75b90926f71e1ef765e6e37c5b947dcbd4aa2b799492db9cb89a7dcc51035096`;
build metadata is saved in `native-final-build.json` in the same audit directory.

The seeded `onboarding.e2e.ts` case now identifies itself as config-to-dashboard
boot coverage. Actual picker/Confirm coverage requires the operator bridge in
`onboarding-indexing-native.e2e.ts`; absence is an explicit capability skip.
