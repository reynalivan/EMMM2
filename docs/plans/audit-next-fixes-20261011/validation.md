# Settings and conflict fix validation

## Implemented outcome

All seven authorized findings and three verified cleanup candidates are implemented.
No dependency, worktree, branch or commit was added. Unrelated working-tree changes
are preserved. Production update installation and credential changes are outside
this validation.

- Browser homepage drafts, mutation variables and cache publication retain their
  original game. Pending/unavailable reads cannot save or reset another game's URL;
  late completions do not erase newer drafts.
- Conflict native cancel is prevented while submit/ignore work is pending. Idle
  close still updates its controlled owner. Winner/overlap tests now use the live
  mod-set API; the test-only wrapper/private forwarding helper are removed.
- The existing Zustand dependency owns updater state beyond GeneralTab remounts.
  Checks/install share frontend admission and progress callbacks are bound to the
  originating operation. One backend RAII guard covers initialization, check,
  download, install and restart, releasing on failure or dropped futures.
- AI endpoint drafts survive unrelated settings updates. Earlier save/remove-key
  completions only clear the submitted edit revision; later drafts remain intact.
- Custom theme validation rejects built-in IDs and their case variants. New defaults
  use Onyx; stored `dark` remains unchanged. Successful metadata determines whether
  it is legacy or a real custom theme. Pending/error metadata cannot prove absence,
  including retained cache data; the picker retains the actual stored selection.
- Metadata directory/entry/read failures are actionable errors with Retry in General.
  Invalid unrelated definitions are deliberately skippable; invalid `dark` is never
  silently interpreted as absence. Windows `dark.JSON` remains discoverable.
- Unused save/delete/export custom-theme IPC is removed from source, registry,
  allowlist and actual Specta export. Active list/load/import/template paths and
  their bounded validation/atomic persistence remain. Five unused DaisyUI aliases
  are removed while semantic `--color-*` output remains.

## Executed checks

- Frontend regressions reproduced before fixes: seven draft failures, three updater
  failures, three conflict cancellation failures, legacy/theme alias failures,
  retained-empty-cache refresh/error failures and two incorrect picker selections.
- Fourteen-file frontend suite passed **87 tests**; two additional picker
  regressions then passed in the **13-test GeneralTab** run, **89 distinct tests**.
- **26 targeted Rust tests passed**: 16 theme tests, four updater admission tests,
  one fresh-default/stored-dark service test, three ConfigService integration tests,
  and two registry/Specta export tests. Filters have no ignored cases.
- Reserved-ID execution RED was captured by removing only the owner's validator
  check after the test had been written; restoring the check passed. Real directory
  failure and Windows `share_mode(0)` lock tests failed before discovery error fixes,
  then passed after recovery. Uppercase extension test failed before the one-line
  case-insensitive filter and subsequently passed.
- App and E2E typechecks, scoped ESLint/architecture lint, Prettier, rustfmt and
  scoped diff checks passed. Strict Cargo Clippy passed all targets with
  `readonly-audit`, one job and warnings denied.
- Independent review found the discovery error/cache and uppercase-extension gaps;
  both were repaired. The discovery repair was reviewed again without findings.
- Initial Cargo invocation from repository root selected the wrong SQLx environment
  and reported unrelated schema errors plus the expected missing updater guard.
  Subsequent Cargo commands ran from `src-tauri` using its offline config; no schema
  or application database repair was performed.

## Native verification

Six distinct cases passed across fresh focused private debug builds of
`settings-conflict-audit.e2e.ts`: browser isolation, AI draft retention through
actual enabled-only IPC, legacy/custom dark, native idle Escape/reopen and the
narrowed theme command surface. The initial run passed four and failed two because
the test looked for GameSelector while in Settings. TopBar intentionally hides it
there. The corrected ordinary UI flow switches from Mods and reopens Settings;
the two focused rerun cases passed. No app UI was changed to accommodate the test.

Both runs checked 1,416 source inputs with zero drift. The final private binary
SHA-256 is `92b1f8ed141bfc75fce8dd3232b024d5bf63e8308b1daf8e1406697249b8d273`.
Each run confirmed owned native process-tree shutdown and removed two scheduled
game fixtures. The exact private theme fixture was removed after the first run,
only after verifying its path and byte hash. The second run did not create it.

Pending context/request ordering, busy-cancel prevention, and credential/update
job interleavings are deterministic unit-test scope. Actual updater installation,
credential vault writes, OS drag/input replay and performance are not certified
by these native cases.

The custom theme fixture uses only the verified private app-data identifier. Its
exact path/content hash is recorded in `owned-theme.json` for cleanup after confirmed
native process shutdown. Real user themes, credentials and game libraries are never
modified by these scenarios.

## Evidence

Ignored `.audit-output/audit-next-fixes-20261011/` retains build-source manifests,
`native.log`, `native-rerun.log`, `native-final-verified.json` and
`theme-cleanup.json`. The first
1,415-input snapshot predates the final uppercase/picker fixes and is historical.
The final freeze snapshot contains 1,415 inputs; the startup task subsequently
added one independent `check-bootstrap-bundle.mjs` validation script, without Vite,
package or build-hook changes. Native input capture accounted for that authorized
addition and the test-only navigation correction; the historical snapshots were
preserved rather than silently overwritten.
