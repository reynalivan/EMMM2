# Watcher consolidation execution

## Scope and baseline

Approved by the user's “eksekusi” request on 10 October 2026. Existing dirty
source was copied to `.audit-output/watcher-refactor-20261010/baseline/` before
editing; that snapshot, not HEAD, is this execution's comparison baseline.
Initial focused watcher suite: 84 passed, no failures or ignored cases.
Unrelated journal, collection and indexing work in the shared checkout is preserved.
Scoped baseline/current hashes are in [execution_source_snapshot.json](execution_source_snapshot.json).

## Implemented boundaries

- P1: one ordered endpoint policy supplies authority relevance and delivery.
  Historical path coverage is lexical; only content classification captures
  explicit directory/missing/unavailable facts. Echo matching still receives
  the original event kind and complete ordered path list.
- P2: value-only binding evidence is separate from actual session resources.
  Candidate and current reuse retain their mode checks; inactive lookup,
  discard and handoff share coverage/health/receiver eligibility and return
  the original session and bounded receiver.
- P3: one cancellation-safe event/work/absolute-deadline wait boundary. The
  owning task's receiver retirement guard remains outside the wait. Dirty
  generations remain demand; a clean work wake preserves backoff.
- P4: one runtime adapter owns Tauri orchestration, activation publication,
  onboarding orchestration, event processing and runtime enqueue. Core
  lifecycle contains no Tauri imports or adapter back-edge. Activation imports
  runtime; external callers use `workspace::api` without an application shim.
- P5: backend guide, focused cancellation/parity fixtures and a native
  external-change/switch/reuse spec added. Verification status follows below.

All 20 baseline lifecycle tests remain exactly once across core and adapter.
No new IPC, dependency, scheduler, coordinator, production queue, schema or
journal change was introduced by this refactor.

## Review and evidence

Independent read-only review preserved activation's held acceptance envelope,
ordinary post-reconcile runtime enqueue, onboarding raw lease/claim/drain, and
session-specific receiver Drop. Review identified two binding parity regressions:
unavailable configured runtime-parent identity and equivalent Windows inactive
path spelling. Regression fixtures precede their corrections.

A suspected final-publication cache defect was disproved: the new capture-then-
invalidate characterization already passed because `authority_catch_up` reads
the real shared cache validity. No redundant production cache check was added.

Executed checks:

| Check                                                              | Result                                  |
| ------------------------------------------------------------------ | --------------------------------------- |
| Workspace Rust suite, including watcher/runtime/cache              | 275 passed, 3 manual benchmarks ignored |
| Reconciliation orchestrator Rust suite                             | 85 passed, 1 manual benchmark ignored   |
| Frontend watcher/activation, 9 files                               | 67 passed                               |
| Strict Clippy `--lib --tests --locked --offline -- -D warnings`    | Passed                                  |
| TypeScript `tsc --noEmit`                                          | Passed                                  |
| New native spec ESLint/Prettier, scoped Rust formatting/whitespace | Passed                                  |
| Core dependency and facade/caller direction checks                 | Passed                                  |
| Fresh isolated native watcher-consolidation + startup-reopen       | 3 cases passed across 2 specs           |

The two reviewed binding regressions failed before their fixes and passed in
the integrated suite. Two new cancellation assertions initially expected
authority deletion; they now correctly assert retained high-water mark,
rejected late authority resurrection and blocked mutation readiness. Their
failure required a test correction, not a production change.

Native run rebuilt the embedded frontend and Windows debug/custom-protocol
executable using `tauri.e2e.conf.json`, verified private identifier
`com.reynalivan.emmm.e2e` and binary hash, and ran WebView2 154.0.4258.62.
The watcher spec projected external create/rename/remove without manual
reconcile, then verified inactive catch-up, same-game reuse and switch-back.
Startup's 301-mod fixture restarted the real process three times; every restart
had exactly one watcher install and one startup baseline, with no duplicate
activation scan. Both owned process trees stopped before cleanup; three
scheduled private game fixtures were removed. No production library/DB was used.

## Acceptance gap closure

The previously recorded full-loop suppression gap is closed by the fresh
private Windows run of `watcher-suppression-native.e2e.ts` through
`wdio.watcher-verification.conf.ts`. One native case covers three controlled
scenarios using the real Wry `AppHandle`, observer, `process_event_loop`, SQL,
result emitter and runtime queue/publication. No miniature consumer substitutes
for the production loop.

| Scenario                                             | Accepted/emitted revision | Exact runtime generation | Artifact  |
| ---------------------------------------------------- | ------------------------- | ------------------------ | --------- |
| Last-owner release; first owner retained suppression | 5 → 6                     | 3                        | Published |
| Real 10-second echo expiry                           | 6 → 7                     | 4                        | Published |
| Discard a natively observed live echo before expiry  | 7 → 8                     | 5                        | Published |

All scenarios retain actual session 5, consume a Work wake before the parked
retry deadline, converge without a second rename or refocus, and prove both
leases available at ordinary enqueue. Final projection has three mods, one
enabled. Runtime ends `needs_manual_reload` because the fixture has no verified
running game/reload target; artifact publication succeeds in all three cases.
Actual game input/reload is not claimed by this test.

The `watcher-verification` Cargo feature is absent from normal builds. It
requires the private debug identifier, canonical nonce-derived temp child,
regular UUID ownership marker, and frozen native identity/marker revalidation.
UI setup precedes an arm file outside Mods. Native directory writes use the
existing no-overwrite rename while holding the real operation lease. Passive
hooks link accepted revision to the exact returned enqueue generation; they do
not inject authority, fake publication, alter clocks/TTL or expose public IPC.

Executed: fresh embedded/frontend native feature build; Windows WebView2
154.0.4258.62, session `cd033ef8c5eee128db9e14b74287aea5`, 1 native case passed
(25.2 seconds). Owned process-tree teardown completed and one private fixture
was removed. Evidence retained in
[native-report.json](../../../.audit-output/watcher-gap-verification/native-report.json).
Source is captured in [verification_source_snapshot.json](verification_source_snapshot.json).
Two safety tests, 80 core watcher tests, 19 runtime tests, strict feature Clippy, app/E2E
typechecks, scoped ESLint/format/whitespace and Tauri-free core checks pass.

Independent review corrected live-abort, coalesced-wake and revision/generation
correlation weaknesses before acceptance. An earlier diagnostic run failed its
barrier and is not counted as coverage. Native timing/external replacement are
still platform risks, not a guarantee of perpetual filesystem coverage.
