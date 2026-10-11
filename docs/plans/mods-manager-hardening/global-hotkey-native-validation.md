# Native global hotkey verification

## Scope

OS keyboard input must reach the private EMMM global-shortcut callback; invoking
workspace actions over IPC or dispatching DOM keyboard events is not coverage.
All Mods, collections and settings writes belong to owned synthetic fixtures.

The default receiver mode tests Safe Mode on/off, three-preset next/previous and
both wraps, overlay refusal/replay/registration restoration, configured game
stopped, and disabled shortcuts. It also requires native minimized state for
the first workspace action.

The explicit foreground-game mode keeps Genshin as keyboard source while
configured `game_exe` remains the fake fixture executable. This prevents runtime
reload from being synthesized into the actual game. It runs workspace actions
and the configured-process-stopped case; focused-overlay and direct disabled-key
receipt are explicitly skipped, not claimed as coverage.

## Safety and evidence

- Fresh debug build through the standard private-identifier/hash/driver harness.
- Test chords Ctrl+Alt+Shift+F13–F16; no shipped F5/F7/F10 input.
- Computer Use selects only returned windows matching the current request.
- UUID-bound completion requires fresh owned observer snapshots before/after
  input and continuous matching foreground evidence sampled at 100 ms.
- Disk spelling, projection counts, runtime collection and Safe Mode are read
  back after actions. Three different states distinguish Next from Previous.
- Receiver input receipts retain modifiers and foreground PID. Logs alone do
  not prove that the receiver processed replay.
- Helpers stop through owned markers; fixture removal follows verified native
  process-tree shutdown. Installed EMMM and game library are not reset.
- Foreground-game PID/executable/window ownership must be reverified immediately
  before input. No process termination or focus transfer of the actual game.

## Results so far

Source typecheck, scoped ESLint, formatting and whitespace checks pass. The
receiver compiles with warnings-as-errors. Standalone 7/35-second lifetime
probes exit normally after controlled stop.

Receiver publication failed under a concurrent Windows reader. A held-read
probe reproduces RED exit 1; bounded coalesced publication for Win32 errors
32/33/1175 yields GREEN exit 0, no stderr, fresh snapshot 109 ms after unlock.
Unrecognized or continuously failing publication still fails explicitly.

Attempts A–C under `.audit-output/global-hotkeys-native-20261010-*` failed at
operator/receiver setup, not product hotkey execution. Attempt D keeps receivers
alive but Computer Use cannot activate them while Genshin is foreground; all
five cases fail or are aborted with **zero OS keystrokes sent**. These are not
passing smoke tests. Owned receivers/fixtures were stopped and cleaned.

Foreground-game attempt E (`.audit-output/global-hotkeys-native-20261011-e/`)
starts with the private fixture/observers ready, but the explicit Genshin process
and window disappear before input. Result: 0 passed, 3 failed/aborted, 2 explicitly
skipped; **zero OS keystrokes sent**. Both observers exit 0 and the owned fixture
is removed after native shutdown. This does not establish global hotkey success
or failure. Continuation requires the user's foreground game to be running again.
Actual 3DMigoto rendering remains unverified.

### Attempt F: real OS input on owned Windows applications

Fresh standard private debug build on 2026-10-11 Jakarta; executable SHA-256
`29eb5ba879dc614a12b38961e0f668b4b547998a5312192d5ef996cbf9c09c22`.
The native spec completed with **4 passed, 1 failed, 0 skipped**. Twelve chords
were actually sent using Computer Use; eleven successful steps were recorded.

- Safe Mode on/off passed with EMMM natively minimized and the other receiver
  foreground. Disk spelling, projection counts and stored state matched.
- Next/previous, the distinct third preset and both wrap directions passed.
  The other application retained focus and received no automatic reload F17.
- Preset cycling passed after the configured fixture game process exited.
- Disabled hotkeys passed: the receiver received the complete Ctrl+Alt+Shift+F13
  chord, while workspace state and disk stayed unchanged.
- Background overlay refusal passed. Focused overlay delivered one F16 and the
  application logged one successful replay/restored registration, but the
  receiver recorded `control=false`, `alt=true`, `shift=true`. The exact chord
  assertion failed. The second focused replay was not reached; registration
  restoration is therefore not certified by repeated overlay input.

Evidence: `.audit-output/global-hotkeys-native-20261011-f.log` and the sibling
directory containing `report.json`, twelve completion receipts,
`overlay-game-status.json`, `hotkey-log-excerpt.txt`, `receiver-exits.json`, and
`validation-summary.json` with source hashes. Both owned helpers exited 0 without
stderr after explicit stop markers. The harness verified native process-tree
shutdown and removed its one fixture; driver ports and compiler/app/helper
processes were clear when the shared slot was released.

This proves global workspace dispatch through real OS input, not correctness in
every foreground application. Actual Genshin was absent and was not launched;
real-game foreground and 3DMigoto rendering remain untested. The overlay failure
is observed, but its root cause is not yet proven. A release/replay race is a
hypothesis: the manager awaits a shortcut release notification, while replay
presses modifiers separately without an explicit physical-modifier release
barrier. No production fix or assertion relaxation was made during validation.

### Attempt G: overlay-only rerun blocked in setup

Fresh standard private debug build and passive receiver warning-as-error
compilation passed. Only the existing overlay test was selected with
`--mochaOpts.grep overlay`. Result: **before-all failure, 0 cases passed, 0 OS
chords sent**; neither focused overlay replay nor registration restoration was
retested.

At spec line 429, preparing the third preset required Unsafe enabled and Safe
disabled. This is before creating that preset and before `apply_collection`.
Read-only inspection of the private DB showed only A All/B Safe, with Safe
enabled and Unsafe disabled; no collection apply task existed. The owned
bulk-toggle journal contained a completed initial disable and a later enable
operation marked RolledBack with its rename step Skipped. Watcher logs reported
directory identity maintenance `PermissionDenied` before that operation. These
observations do not establish why the bulk operation rolled back or connect it
causally to another task's source changes. The setup currently does not retain
or assert per-item bulk command results, so that missing evidence limits diagnosis.

The passive receiver now records bounded down/up transitions for fixture F13–F18
and modifier keys only. It still has no hook, focus transfer or input injection;
existing receipt assertions remain intact. Both helpers exited 0 without stderr,
the owned native tree stopped and its one fixture was removed. Shared compiler,
driver ports and GUI slot were released; no additional run was started.

Evidence: `.audit-output/global-hotkeys-native-20261011-g.log` and the sibling
directory with private DB/journal precondition snapshots, run-local log excerpt,
fixture ownership and exit evidence. No real Genshin or mod-library access.

Dependency source inspection confirms `global-hotkey` 0.7.0 checks only the main
key for its release event; `enigo` 0.6.1 sends each modifier/main-key operation
separately. This supports, but does not prove, a physical-release/replay race.
Windows documents interference from keys already pressed during synthetic
input: [SendInput remarks](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput).

## Run

Latest continuation: run J reproduced the release/replay race with passive
modifier event evidence. The production fix waits for chord release and sends a
balanced Windows input batch. Run K passed background overlay refusal and five
focused full-modifier replays, but its overall result was **1 pass / 5 failures**:
watcher identity coverage failed and a status read hit EBUSY. Three setup bulk
commands succeeded; the later capture/toggle stress loop did not start. Both
helpers exited normally and the owned fixture was removed.

Review corrections for partial cleanup, locale-independent virtual keys and
semantic modifier-alias deduplication pass 49 focused Rust tests and strict
all-target Clippy, with no remaining finding in their independent review.
Run L includes those corrections: five full-modifier focused replays, background
overlay refusal, minimized-app Safe Mode on/off and disabled behavior pass.
Its overall result is **3 pass / 3 fail**. A runtime-parent File.Replace temporary
file caused metadata Windows error 5, invalidating Mods watcher coverage before
preset/stress tests. The scope fix now passes 284 workspace tests (4 existing
profiling/benchmark cases ignored), strict Clippy and independent review; a fresh
native repeat remains pending. Physical held modifiers, focus loss and actual
Genshin also remain incomplete. No full-safety claim is made.
See the [remediation plan](hotkey-replay-remediation-plan.md#run-k-and-review-checkpoint).

Run G's journal mechanism is now resolved: a second watcher-proof check rejected
the already-journaled but untouched plan (`authority_dropped=true`), so it settled
Skipped/RolledBack without any filesystem rename. The originating watcher OS
error is still unproven; see the
[bulk abort evidence](hotkey-replay-remediation-plan.md#historical-bulk-abort-mechanism-resolved).
Run L's separately proven watcher path is recorded in
[watcher failure provenance](hotkey-replay-remediation-plan.md#run-l-watcher-failure-provenance).

Set `EMMM_E2E_HOTKEY_OPERATOR_DIR` to a fresh absolute artifact directory, then:

```powershell
rtk proxy corepack pnpm exec wdio run wdio.conf.ts --spec tests/e2e/specs/global-hotkeys-native.e2e.ts
```

The operator reads `request.json`, verifies its fixture marker and target window,
observes foreground, presses only its requested chord via Computer Use, refreshes,
and publishes matching completion snapshots. A failure must be reported as an
error completion, never success. Without an operator directory the spec skips.

Foreground source requires the explicit JSON environment contract
`EMMM_E2E_HOTKEY_FOREGROUND_GAME` with the live Genshin PID, executable and title;
it grants no access to game-library data and is not an input authorization itself.

References: [Windows Forms keyboard handling](https://learn.microsoft.com/en-us/dotnet/desktop/winforms/input-keyboard/how-to-handle-forms),
[Windows replacement error semantics](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew).
