# Client smoke audit — 2026-10-07

## Scope and authority

User requests positive, negative and edge-case client end-to-end testing, including every reachable button/action. Report-only product audit: application implementation is not changed. Test scripts and evidence may be added. User explicitly authorizes temporary QA fixtures in the isolated E2E database and marked OS-temp folders, followed by owned cleanup; production library/database/credentials are excluded.

## Execution layers

1. Run the actual frontend in `pnpm dev:demo` with Playwright. Enumerate visible controls on every view, tab, menu and reachable dialog. Exercise every unique safe action and option, preserving per-control positive/negative/edge verdicts. In-memory demo state resets on reload by contract; this is not persistence or filesystem proof.
2. Add focused native WDIO client smoke specs using the existing fresh-build/app-identifier gates. UI clicks/fills are the action under test; IPC is only fixture setup and result verification. Assert real QA disk names, database/read-model persistence, and no mutation outside owned paths.
3. Record unavailable dependencies explicitly. Do not count a demo missing-handler rejection, a mocked success, or a direct IPC-only test as a successful native UI path. Native OS file pickers, external game execution, paid AI/network calls, app installation/restart, system-wide registrations and live production updater/catalog downloads are not executed without suitable isolated support.

## Ownership

| Executor                     | Surface                                                                                            | Environment                                         |
| ---------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Main                         | Demo shell, all views, menus/dialogs, live control inventory, console/network, consolidated report | New owned Playwright tab at `http://127.0.0.1:5173` |
| Settings native spec author  | All ten Settings tabs and safe form/CRUD actions                                                   | `client-settings-smoke.e2e.ts`                      |
| Workspace native spec author | Dashboard/Mods/Preview/Collections, selection/switch/rename/edit/collision                         | `client-workspace-smoke.e2e.ts`                     |
| Secondary native spec author | Inbox/Optimizer/Discover/Downloads safe fixture actions                                            | `client-secondary-smoke.e2e.ts`                     |

Authors do not run browsers/builds/tests. Main runs native specs serially against `com.reynalivan.emmm.e2e`, never a production binary. Fixtures use `EMMM_QA*` temp roots, small classifier-valid mod sets, fake nonexecuted loader paths and safe QA names. Exact fixture counts will be recorded from the final scripts before execution.

## Cases per action

- Positive: normal click, correct visible result, expected read-model/disk effect where native.
- Negative: empty/whitespace, invalid or duplicate input, missing prerequisites, operation rejection, cancel/backdrop/Escape, failure feedback and control recovery.
- Edge: Unicode/long/HTML-like text as appropriate, boundary values, repeated/double clicks, rapid reverse switching, game/view changes, disabled ancestors, physical destination collision, saved versus current collection state.
- Verify fresh DOM after changes; inspect console/network errors and actual authoritative state. No forced reconcile to manufacture convergence.
- Restore language to English and keep persisted global hotkeys and telemetry disabled. Temporary hotkey-enabled drafts are allowed only without saving them.
- Destructive actions target QA fixtures only. Do not invoke guard tests against system/user paths or empty the Windows Recycle Bin. Recycle-bin side effects, if needed, must be explicitly recorded.

## Verdicts and evidence

Use `PASS`, `FAIL`, `BLOCKED` and `NOT RUN` per action/case. Capture exact error, repro, source mechanism, environment, and whether the result is product/native versus demo-only. Keep inventory totals and distinguish unique action types from repeated rows. Save failure DOM/screenshots under owned artifact locations. Verify findings against source and fresh pointer-driven reproduction before calling them product bugs.

The final report must state what remains untested. No claim of “all features passed” from route mounting, demo controls, prior unit tests, or IPC-only cases. No commit/push or installer operation is part of this audit.
