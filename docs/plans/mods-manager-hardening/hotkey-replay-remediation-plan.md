# Global hotkey replay remediation

User approved this sequence on 2026-10-11 (Asia/Jakarta).

## Scope and boundaries

Keep global Safe Mode/preset dispatch, overlay focus restrictions and all mutation
ownership/watcher guards. Use private native fixtures only; no installed-app or
real-library changes. Coordinate compiler/native/GUI slots with the startup audit.

## Sequence and acceptance

1. Retain and assert each setup bulk result, including request operation ID,
   failures, cancellation and progress counts. Retain passive receiver status
   through teardown. A setup failure must identify the actual command outcome,
   not silently become a folder timeout. Typecheck/lint/format this instrumentation.
2. Fresh native run: reproduce setup failure and inspect the result before any
   fix. PermissionDenied is not assumed to be a sharing violation. Preserve
   original root/participant identity and rollback guarantees.
3. Reproduce overlay with passive modifier/down/up and message metadata evidence.
   If physical-release/replay interference is confirmed, implement a bounded
   release barrier and coherent chord dispatch with last-moment focus validation
   and registration restoration on every exit. No focus transfer or arbitrary
   unconditional sleep; no replacement keyboard hook or privilege elevation.
4. Regression-first verification for the confirmed fix: fast/held modifiers,
   repeated overlay, focus loss, disabled input, global minimized Safe Mode and
   preset directions/wrap. Run relevant Rust tests, strict lint/format/typechecks
   and fresh native fixture checks; review material input/ownership changes.

## Current evidence

Run F: 4 passed/1 failed, focused overlay Ctrl missing. Run G: setup third preset
blocked before OS input; enable Unsafe RolledBack/Skipped. Causes remain unproven.
Diagnostic changes alone do not constitute a production fix or passing native
coverage. Real Genshin foreground/rendering remain distinct unverified scenarios.

## Diagnostic preparation

Setup now retains each generated operation request and its complete typed
BulkResult (or IPC rejection), asserting successes, failures, cancellation and
processed/unprocessed accounting immediately. Teardown retains each owned
receiver's final status as well as its exit record. Passive modifier transitions
include thread message metadata encoded losslessly as hexadecimal; Enigo 0.6.1
uses marker 100 by default, but metadata is diagnostic evidence, not authority.

E2E TypeScript, scoped ESLint, Prettier and whitespace checks passed. C# changes
were subsequently compiled with warnings-as-errors in run I. No additional OS
input beyond historical run F has been sent.

Read-only discovery confirms zero applied sequences intentionally settle as
Skipped/RolledBack with no operation-level last_error; the per-step error resides
in BulkResult.failures. Watcher cache PermissionDenied currently loses the API,
path and raw OS code, so it cannot establish the cause of that step failure.

## Run H build gate

The requested fresh debug native run stopped in onPrepare before app, driver,
receiver or fixture creation. Frontend TypeScript rejected another task's
`useFileDrop.test.tsx:46:54` unlisten mock as not assignable to `UnlistenFn`.
Evidence is `.audit-output/global-hotkeys-native-20261011-h.log`. No OS key was
sent and no setup outcomes were produced. The file owner was notified to repair
the typed mock without bypasses; the shared slot was released for coordination.

Further read-only investigation disproves repeated `intentRevision: 1` alone as
the cause: current admission accepts equal revisions (`latest <= revision`). A
strictly newer intent would be needed for supersession, and no such evidence has
been observed. Do not change fixture revisions as a speculative workaround.

## Run I after human-prioritized handoff

The human prioritized hotkey work at the next safe shared checkpoint. Fresh
private debug frontend/native build and receiver warnings-as-errors compilation
passed. All three setup bulk commands returned one success, no failures,
no cancellation and complete processed accounting; setup and restoring A All
completed. The previous rollback was not reproduced and no production fix was made.

The operator could not activate the receiver. Fresh selection/retry and passive
foreground evidence identified PID 80996; read-only CIM identified `LockApp.exe`.
Stop Windows automation immediately for the locked desktop; do not interact with
LockApp, authentication or security settings. Report an error completion rather
than keyboard success. Zero OS chords, zero received keys/transitions; the single
overlay case failed at the operator prerequisite, not at overlay replay.

Both owned helpers exited 0 without stderr through stop markers. Native process
tree shutdown and one-fixture removal were confirmed; driver ports/processes were
clear. Release CPU/compiler work to the startup task while preserving the human's
next unlocked native priority. One shared human-unlock request is required before
another run. Evidence: `.audit-output/global-hotkeys-native-20261011-i.log` and
the sibling directory containing bulk results, final statuses and validation
summary (build SHA-256 `e50c421f023e5db7216d571c4db8b984b115b9749296e3e8a6d5bc883ac86bb3`).

## Run J: confirmed replay race

Fresh private native run reproduced the missing Ctrl after all three setup bulk
commands succeeded. At 07:39:16 UTC the passive game receiver recorded replay
Ctrl-down (marker 100) at .453, original Ctrl-up (marker 0) at .457, then replay
F16-down at .459 with Ctrl false. The original key release interleaves separate
Enigo calls; this establishes the cause rather than just the symptom. Background
overlay rejection succeeded; focused replay failed. Helpers exited 0 and the
owned native tree/fixture were cleaned.

The scoped fix waits for the complete physical chord with a deadline, then checks
the original game/executable/binding and enabled setting. Windows submits one
balanced input batch after a last focus/released-key check; partial insertion
attempts cleanup, and registration restoration still runs after replay failure.
Non-Windows retains Enigo. Focused Rust regressions pass 46/46; fresh native GREEN
and independent review remain pending. Tests now request five focused replays and
eight capture/toggle cycles (sixteen recorded bulk operations).

Read-only collection/Windows pin audit found no retained collection pin explaining
run G. Its historical per-item error was not saved, so that cause cannot be
recovered from journal settlement alone. No speculative bulk production fix was
made; the next native test retains complete per-item outcomes.

## Run K and review checkpoint

Fresh private native build: **1 case passed, 5 failed**. Background overlay
refusal and all five focused replays passed, with Ctrl/Alt/Shift present on each
F16 receipt. Both owned helpers exited 0 and the fixture was removed after native
shutdown. Evidence: `.audit-output/global-hotkeys-native-20261011-k/`, including
`private-app-log.txt`, completion receipts and `receiver-exits.json`.

This is not a full native GREEN. Watcher directory identity coverage became
invalid with PermissionDenied at 07:49:15 UTC, then Safe Mode off and preset
commands were rejected by the authority guard. Failures repeated while idle.
The eight capture/toggle cycles never started because their initial collection
apply was rejected. A direct status-file read also hit EBUSY during atomic
receiver publication; bounded read retry is still pending. Keep state assertions
and watcher guards intact.

An isolated locked regular-file regression passed. The existing watcher log
retains only ErrorKind, not API stage/path/raw Windows code, so simple file-lock
and atomic-replacement hypotheses remain unproven. Cache maintenance precedes
event relevance filtering; unrelated runtime-parent file activity is a candidate
to investigate, not an established cause. Run G's abort mechanism was subsequently
established from the retained proof-rejection log and journal; see below.

Independent review found partial SendInput cleanup releasing keys not inserted,
and locale-sensitive ASCII conversion differing from registered virtual keys.
Both were corrected, with 48 hotkey tests and strict all-target Clippy passing.
The mapping regression now uses reachable `Ctrl+1` with an injected shifted
layout converter, not unreachable uppercase input. Final review additionally
found valid modifier aliases (`Ctrl+Control`) need semantic deduplication for
the cleanup invariant. That regression failed before the fix; the parser now
deduplicates semantic modifiers while preserving their first occurrence order.
All 49 focused hotkey tests, strict all-target Clippy, targeted rustfmt checks
and `git diff --check` pass. Independent review reports no remaining finding
in the replay-correction scope. No fresh native run includes these review
corrections yet.

Source/native access was handed back to the concurrent startup-profile task
after those checks. Pending follow-up is narrow watcher error provenance
(operation, path and raw OS code), bounded receiver-read retry for EBUSY, then
a fresh native run including capture/toggle stress. Physical held modifiers,
focus loss and actual Genshin foreground still require their own validation.

Physical held-modifier/focus-loss scenarios and real Genshin foreground/rendering
remain unverified. Source/compiler/native work is coordinated with the startup
profiling owner; do not treat its unrelated benchmark as hotkey certification.

## Historical bulk abort mechanism resolved

Further evidence from run G identifies the immediate failure path. The journal
was created at 05:53:52.624 with one planned rename. At 05:53:53 the retained log
records `toggle storage proof rejected`, with `authority_dropped=true` while
session/root/index/trust still match. Bulk toggle validates this proof once before
creating the journal and again in `DiskMutationLease::from_toggle_scope_guard`.
Failure of the second check calls `abort_unapplied`, which marks untouched steps
Skipped and the operation RolledBack with database projection NotStarted.

Thus G's journal is an **unapplied-plan abort**, not a filesystem rename followed
by a physical rollback. The command exits before constructing per-item results;
the newer harness retains IPC rejection as well as BulkResult. Keep this guard.
The preceding watcher PermissionDenied is consistent with the lost coverage,
but its originating OS operation/path/raw error remains unknown.

## Run L: watcher failure provenance

The first-failure diagnostic now records cache stage, path, ErrorKind and raw OS
code without changing invalidation. Receiver reads retry only EBUSY twice at
50 ms; malformed JSON and identity failures still reject. All 12 identity-cache
tests, strict all-target Clippy, E2E typecheck/lint and formatting pass.

Fresh native L includes the final replay review corrections. Safe Mode on/off
from the other foreground app with EMMM minimized, background overlay refusal,
five focused full-modifier replays and disabled-hotkey behavior pass. Overall:
**3 cases pass / 3 fail**. Preset wrapping, stopped-game cycling and bulk capture
stress remain blocked by the watcher guard. No receiver-read EBUSY failure was
reported. Both owned receivers exited 0, the harness stopped its native tree and
removed the fixture; a follow-up process inventory found no owned app/helper.

At 09:04:29 UTC the first cache error was `operation=metadata`, Windows code 5,
on the runtime-parent sibling `game.status.json~RFc57a8ae.TMP`. This transient
File.Replace artifact is outside Mods and is not the runtime INI. Cache scope
admits it before event relevance filtering, permanently invalidating coverage.
The next PrevPreset at 09:04:39 was rejected for watcher repair. This establishes
L's failure chain; it does not retroactively identify G's missing raw error.

The scoped correction is now implemented: cache roots retain RecursiveMode;
recursive Mods coverage wins over a surrounding nonrecursive parent, whose
refresh caches only itself without enumerating siblings or deleting nested Mods
identities. Public subtree removal remains unchanged. Two deterministic
regressions failed before the fix, then passed. Tests also retain recursive Mods
failure invalidation, missing roots in either mode and runtime INI Modify/From/To
authority/classification. Independent review found no remaining scoped defect.

Validation after the scope fix: **284 workspace tests pass / 4 existing tests
ignored**, strict all-target Clippy, scoped rustfmt and diff checks pass. Ignored
cases are the 1,000-object projection benchmark, 100k listing benchmark, 10k
folder scan and owned-avatar catalog profiler; they are not new ignored
regressions. The fix has not yet had a fresh native repeat. Source/compiler/native
ownership was handed back for the coordinated cleanup/startup validation.
Actual held modifiers, focus loss and Genshin also remain untested.

Evidence: `.audit-output/global-hotkeys-native-20261011-l.log` and its sibling
directory, including `private-app-log.txt`, reports and completion receipts.

Evidence: G `app-log-excerpt.txt:71-72` and `private-journal-precondition.json`;
code: `mod_bulk_cmds.rs` proof checks / lease creation,
`orchestrator/state.rs::from_toggle_scope_guard`,
`mutation/journal.rs::abort_unapplied`.
