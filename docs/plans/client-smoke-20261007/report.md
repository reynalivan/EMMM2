# Client end-to-end smoke audit

Date: 2026-10-07 (Asia/Jakarta). Verdict: **DO NOT SHIP as fully verified**.

## Scope and evidence boundaries

This is a product audit, not authorization to fix the application. This session adds only four client smoke specs and audit documentation. Existing application edits arriving concurrently are preserved, not attributed to this audit.

Baseline: `main` at `fd14e062f14c1c3734cb0f1f8fb6a91053bd43e2`. During testing, other work changed demo commands, Mod Inbox, the import wizard host and app entrypoint. Native runs rebuild a private binary per invocation; the results therefore describe each tested local snapshot, not an unchanged release commit.

- Native UI: real WebView2/WDIO pointer and keyboard actions against `com.reynalivan.emmm.e2e`; IPC only arranges fixtures and verifies authoritative results.
- Disk: only marked OS-temp QA roots; real folder prefixes, sentinels and stored settings checked where a case reaches them.
- Demo: actual application UI, in-memory adapters only. A demo success is not native filesystem, database, secret-vault or network proof.
- Repeated runs are not added together as unique passing cases. Setup/selector failures are separated from application findings.

The requested “every button/action” coverage is **not complete**. Failed earlier steps prevent later substeps, and OS integrations need further isolated support. The matrix below records gaps explicitly.

## Confirmed findings

### F01 — P1: normal Settings saves reject stale revisions after activation

Repro: load the isolated indexed game, open Settings, change a Safety keyword, theme, language or hotkey configuration, and save through the visible controls. Trusted native input/change events occur, but the native value remains unchanged. Safety/Hotkeys report:

> Settings changed since this screen was loaded. Refresh and retry your edit.

The dedicated Close after launch roundtrip succeeds, unlike several whole-settings saves. Source explains the discrepancy:

- `src-tauri/src/modules/settings/application/config/service.rs:207` increments the settings revision on updates, including `set_active_game` at line 226 even for an already active game.
- `src/app/store/appStore/gameSlice.ts:348` changes only `active_game_id` in the cached settings after activation, retaining its previous revision.
- `src/entities/settings/api/settingsQuery.ts:10` uses an indefinitely fresh cache.
- `src/app/store/appStore/gameSlice.ts:410` refreshes the full settings snapshot only for the dedicated auto-close mutation.

No numeric submitted-revision trace is claimed: the initial passive bridge interceptor did not install because Tauri's `invoke` property is non-writable. That observer was removed. Native rejection, unchanged readback, trusted events and source mechanism are the evidence.

Smallest proposed correction: every settings-changing native action publishes the authoritative returned snapshot/revision to the same query cache. Do not blindly retry or drop revision validation.

### F02 — P2: duplicate Mods-path validation is bypassed in Add Game

Native Add Game permits a same-path candidate without displaying the intended duplicate validation. The captured candidate and existing path canonicalize identically; a Windows verbatim-prefix mismatch is ruled out for this reproduction. Registration success itself is not certified because the save also encounters F01.

`src/pages/settings/modals/GameFormModal.tsx:79` configures a Zod resolver, while the duplicate rule exists only in `register('mod_path', { validate })` at line 98. The installed form implementation uses the resolver branch instead of the built-in field validation branch. The Zod schema lacks this duplicate refinement.

Smallest proposed correction: keep the duplicate rule in the actual schema/resolver validation path; retain backend path/identity validation. Same folder name at different physical paths must remain valid.

### F03 — P2: Privacy/Terms Escape does not close the dialog

Reproduced in both browser demo and native UI. The Close button works; reopening and pressing Escape leaves the dialog visible.

`src/pages/settings/components/TrustInformationDialog.tsx:54` declares `<dialog open>` with `aria-modal` and `onCancel`, but does not open a native modal with `showModal`. Therefore the native Escape/cancel behavior is absent; declaring `aria-modal` does not establish it.

Smallest proposed correction: use the existing dialog synchronization primitive, including focus return, rather than a second modal system.

Screenshot: `logs/client-smoke-20261007/settings/2026-10-07T15-38-45-296Z-06-opens_Privacy_Policy__closes_with_its_button__then_closes_a_fresh_dialog_with_Escape.png`.

### F04 — P2: blank Discover address input loses its editing element

Native typing a complete owned localhost URL from a blank address fails before any navigation/consent. Diagnostic DOM shows `#browser-url-input` has become a button containing only `http://1`, no focused input, no native tabs and no requests to the QA server.

`src/pages/browser/components/BrowserToolbar.tsx:62` starts with editing false. At line 139 a recognized address replaces the input with a formatted button whenever editing is false. Only that button sets editing true on focus/click (lines 146–147); the blank input does not. A valid prefix can therefore unmount the field during typing. The native artifact matches this source transition.

Smallest proposed correction: enter edit mode on input focus and retain the input throughout the active edit. Do not treat navigation through a staged-entry workaround as proof that direct typing works.

Artifact: `logs/client-smoke-20261007/browser/2026-10-07T16-19-44-075Z-beforeEach-navigates_owned_native_pages_with_Back_Forward_Reload_Find_Zoom_and_child-tab_recovery.json`.

### F05 — P2: useful backend errors become `[object Object]`

Native theme/language/AI saves show generic or `[object Object]` feedback even though the backend provides an actionable revision error. `src/entities/settings/api/useSettings.ts` interpolates `String(err)` in several handlers.

Smallest proposed correction: use the established typed error formatter, preserving the actionable payload without leaking secrets or private paths.

## Demo-only findings and limitations

- Homepage save/reset followed by leaving and reentering Browser Settings reads `demo-zenless`, not the requested URL. `browserSetHomepage` in `src/demo/commands.ts` stores argument 0 (game ID) rather than argument 1 (URL). Native homepage roundtrip passed. Do not classify this as native persistence failure.
- `useBackgroundIndexingStatus` directly subscribes to Tauri events in demo mode and logs missing `transformCallback` errors. This is a demo/native adapter boundary issue, not proof of a broken native disk watcher.
- Missing demo handlers reject explicitly: auto-close, custom theme import/export, updater/external links, hotkey save, maintenance/cache, import decisions/cancel. Some adapters were changed by concurrent work after initial observations; those later changes are not certified automatically.
- The in-app QA browser twice timed out while native builds and workspace changes were active. Closing only the owned tab and opening another briefly restored access. No production freeze diagnosis is inferred from that transport failure.

## Coverage matrix

| Surface             | Native paths reached successfully                                                                                                                                                                                                                           | Remaining/failed paths                                                                                                                                                                                                 |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dashboard and shell | App Menu navigation, switching between two QA games without mixed rows                                                                                                                                                                                      | Actual game launch, chart interactions, global unfocused hotkeys, full first-run onboarding UI                                                                                                                         |
| Mods Manager        | Object search/clear; mod search/clear, list/sort; single card and preview switch; seven actual rapid switch clicks with final intent checked on disk; two-mod bulk disable/enable/clear; Unicode folder create and duplicate/blank rejection                | Pin persistence, preview Rename, metadata clearing, INI transition guard under investigation; move/reorganize/object CRUD, import pickers, resize/drag gestures, large-library stress                                  |
| Preview             | Switch filesystem roundtrip, gallery/fullscreen navigation and image-delete cancel                                                                                                                                                                          | Positive image deletion/add/paste, external editor/viewer, keybinding overlay in a real game                                                                                                                           |
| Collections         | Blank-name rejection/cancel, Unicode save, rename/cancel with membership preserved                                                                                                                                                                          | Exact apply/cancel/restore and deletion replay recorded below; missing/conflict recovery and global preset cycling not certified                                                                                       |
| Settings            | All ten tabs mounted; Close after launch; blank keyword no-op; incomplete game cancel; homepage save/reset/non-HTTP rejection; retention boundaries 1/365 and invalid 0/366/1.5/empty; logs refresh/levels; maintenance/cache; reset Cancel/Escape/backdrop | F01/F02/F03/F05; later parts of CRUD and AI flows blocked by first save; OS pickers/links, catalog install, updater install/restart, telemetry enabling, real credentials/API, positive reset not executed             |
| Mod Inbox           | Missing directory Create Folder; selected Ready archive/folder review, close/resume, cancel                                                                                                                                                                 | Processed positive actions initially blocked by incorrect relative-path fixture verification; exact replay result below. Arbitrary file chooser, full positive import conflict resolution and bulk scale not certified |
| Storage Optimizer   | Duplicate scan; confidence controls; Keep cancel; Ignore resolve/recover; Keep resolves exactly owned candidate                                                                                                                                             | Stop Scan race: button observed, cancellation toast not proved. Nonexact large scans/partial failure require further controlled fixtures                                                                               |
| Discover            | Blank-tab/menu/library actions reached; address field failure diagnosed                                                                                                                                                                                     | Child-page navigation/download suites initially blocked beforeEach by F04; later workaround results explicitly distinguished below                                                                                     |
| Downloads           | Empty-list refresh, Discover detail-panel open/close/reopen and Mod Inbox navigation                                                                                                                                                                        | Real localhost archive download/metadata/rename/delete-cancel awaits successful native navigation; retry/pause/resume/queue, unknown-size/executable failures not certified                                            |

Demo actions additionally cover dashboard key search, literal special-character empty results, five classification choices, theme choices, Unicode/normalized Safety keywords, hotkey duplicate/reserved drafts and resets, fake AI show/hide/save/remove, and several read-only dialogs. These are UI fixture checks only.

## Run accounting and harness corrections

Details are in `execution-summary.json`. Settings baseline: 27 passing, 15 failing across 42 cases. Three F-key draft failures initially came from the harness sending function keys rather than literal text; corrected tests must be replayed before those become product verdicts. Theme System/Onyx cases fail at the Light priming step, so their target selections were not reached.

Workspace first three attempts stopped in setup. A subsequent 20-case run reached 19 cases: 11 passed, 8 failed and the final case was blocked by a beforeEach missing IPC argument. Proven harness issues were repaired without altering product sources or masking persistence assertions: selector grammar; conditional panel/dialog stale handles; required operation ID; inactive collection fixture; relative runtime Include configuration; collision contract (the responsive switch is intentionally allowed to open resolution, rather than being disabled).

Secondary baseline replay: 4 passed, 3 failed, 2 explicit skipped. Processed setup lacked a review marker, then its disk verification used a relative path as absolute; neither proves a failed product delete. Stop cancellation is inconclusive on the tiny fixture. A two-spec run also hit a transient driver-port handoff failure; subsequent execution uses separate native invocations instead of killing unknown processes.

## Isolation, cleanup and safety

The existing harness rebuilds with the E2E identifier, verifies the application identifier before resetting only its database, and snapshots the native binary. Persisted telemetry and global hotkeys remain disabled. No real game, user library, production DB or credentials were used. The fake credential is an invalid QA string; no paid API request was issued.

The browser fixture seeds only `browser_settings.adblock_last_success_at` in the verified E2E `app.db` to avoid third-party filter downloads, captures the prior value and restores it afterward. This does not certify adblock filter quality. Local HTTP server binds loopback only and is owned by the spec.

Completed harness runs report stopping their owned process tree before removing scheduled marked fixture roots. Duplicate Keep tests may leave recoverable **QA-only items in the Windows Recycle Bin** after root cleanup. The global Recycle Bin was not emptied. Known retained candidates are recorded in execution evidence; an earlier run lacked a candidate log, so complete Recycle Bin cleanup is not claimed.

No product repair, commit, push or installer release was performed by this audit.

## Final focused replays and residual gaps

| Execution                            | Actual result                                                                                         | Meaning                                                                                                                                                        |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Settings baseline                    | 27 passing / 15 failing, 42 cases                                                                     | Includes three subsequently corrected F-key harness failures                                                                                                   |
| Settings focused replay              | Six cases reached: four assertion passes, two assertion failures; driver disconnected during teardown | Cold first save fails with exact stale-revision message; cache-refreshed identical save passes; F5/F6/F8 conflicts pass; duplicate-path validation fails again |
| Workspace first completed action run | 11 passing; eight action failures; last case blocked by beforeEach                                    | Gallery, collection create and rename passed in this snapshot; apply/delete not certified                                                                      |
| Workspace last replay                | 15 reached: nine passing, six failing; five not run after driver disconnect                           | Rapid seven-click latest-wins and bulk two-mod pass again; disabled-ancestor cancel/confirm passes; preview/selection failures remain unresolved               |
| Secondary solo replay                | Four passing / three failing / two skipped                                                            | Ready review/cancel, duplicate actions and empty Downloads pass; Processed verification, Stop cancellation and URL entry fail                                  |
| Localhost Browser                    | No positive case completed                                                                            | Earlier beforeEach blocked at URL entry; the subsequently added staged-entry workaround and separate direct-entry regression are **not rerun**                 |
| Demo browser                         | 55 action/scenario records: 39 PASS / 13 BLOCKED / three FAIL                                         | See `demo-actions.md`; counts are neither unique controls nor native integration proof                                                                         |

Workspace failure classification is deliberately narrow:

- Single-card disk disable succeeds, but its later preview selection disappears before the preview switch is reached in the last replay. Earlier card/preview roundtrip passed; this path is not consistently green.
- Favorite/Pin case stops at one selected mod where two were expected, **before the Pin action**. Pin persistence is not certified, and this failure is not proof that its backend write is broken.
- Preview-menu Rename does not expose the expected grid inline input. Source/dynamic cause remains unproven.
- Metadata replay stops when the preview empties after an author edit, before version/description clearing. The earlier description timeout and later lost preview are not conflated into a confirmed description-save bug.
- INI unsaved transition leaves the next mod selected without the expected guard; subsequent discard/save stages are not reached. Further isolation of watcher/reconciliation effects is needed.
- Physical collision banner and read-only preview render. The test's requirement that a Shared Hash badge be absent was invalid: those advisories can coexist. It was corrected, but the switch→resolution and untouched-sentinel sequence is still **not replayed**. Do not claim a collision pass.

Both last native invocations lost their driver, and a read-only process inventory then showed no native app/driver remaining. Causation is unknown; concurrent work/test interference is a possibility, not a finding about application crashes. Further native replays are stopped pending an uncontended driver environment. Browser pane transport also timed out repeatedly. This prevents completing the requested exhaustive client coverage in this execution.

The cold/cache-refreshed comparison and exact duplicate-path equality flags are saved in `settings-focused-evidence.json`. These are observed native scalars/trusted event counts, not a submitted IPC revision trace.

A read-only isolation/assertion review found no production-state escape in the four specs and identified one test gap: Processed-source deletion also needs physical archive absence, not merely `sourceDeletedAt` and UI state. A bounded exact-owned-path `ENOENT` assertion now covers this test requirement and rethrows other I/O failures. It has not been replayed; the positive deletion path remains uncertified. The review accepts the report only as an incomplete audit, not an all-controls or release gate pass.

The four retained roots were separately verified by exact absolute path, matching run creation time, UUID ownership marker, mock executable content, no reparse descendants and absence of native processes, then removed: `EMMM_QA_workspace_U1qiiM`, `EMMM_QA_workspace_second_BH9YiE`, `EMMM_QA_Settings_Base_jtyRE9`, `EMMM_QA_Settings_Added_Q26uLB`. Absence after removal was checked. They were disposable generated fixtures; deletion is not recoverable through the app. Screenshots/DOM/diagnostics remain in the repository's ignored `logs/client-smoke-20261007/` directory.

The isolated E2E database/cache is retained rather than broadly deleting app data; the existing harness resets that private database at the start of the next spec. Known QA Recycle Bin residuals include candidate folders from `EMMM_QASecondary_Duplicates_u2JLm0` and `EMMM_QASecondary_Duplicates_LoyEzH`, plus an earlier run whose candidate path was not logged. No complete Recycle Bin cleanup is claimed. No real credentials were retained by a successful native QA save.

Only the owned in-app tabs and the verified demo Vite PID were closed. Unrelated browser profiles, WSL/Rancher port owners and other application edits were untouched.

Validation of audit artifacts: main E2E typecheck and focused ESLint on all four specs passed. Fresh native builds completed before the attempted runs; failing smoke cases are preserved. No installer, global hotkey registration, telemetry enablement, third-party navigation, paid API request or production reset was performed.

## Acceptance before declaring all controls verified

Fix confirmed product failures in a separately authorized change, then rerun their original UI regressions plus the affected switch/projection paths. Finish the explicitly missing controls with isolated native support. Keep disk/read-model assertions and report each prerequisite-blocked substep; do not manufacture an all-green result by direct IPC under test or forced reconciliation.
