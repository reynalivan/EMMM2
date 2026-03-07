# Test Cases: App Bootstrap & Initialization (req-01)

## A. Requirement Summary

- **Feature Goal**: Implement a structured and sequenced startup pipeline ensuring single-instance execution, DB migrations, core plugin registration, and correct routing.
- **User Stories**: Single Instance Guard, Database Initialization, Config Status Routing, Plugin Registration, Managed State Registration.
- **Acceptance/Success**: Interactive ≤ 2s, Migration ≤ 500ms, Focus second app ≤ 300ms, Zero panics, 100% correct routing.
- **Main Risks**: DB corruption on concurrent launches, race between UI ready and Backend state, SQLite file permission errors.
- **Gaps / Ambiguities**: OS behavior on LocalAppData read-only access isn't explicitly defined, App relaunch sequence for settings changes requires backend lock release management.

## B. Coverage Matrix

- AC-01.1.1, AC-01.1.3 → TC-01-01 (Single Instance Rescue)
- AC-01.1.2 → TC-01-02 (Happy Path Start)
- AC-01.1.4 → TC-01-03 (Concurrent Launch Guard)
- AC-01.2.1 → TC-01-04 (DB Fresh Install)
- AC-01.2.2 → TC-01-05 (DB Schema Migration)
- AC-01.2.3, AC-01.2.5 → TC-01-06 (DB IO Errors)
- AC-01.2.4 → TC-01-07 (Missing AppData handling)
- AC-01.3.1 → TC-01-08 (Route: Welcome)
- AC-01.3.2 → TC-01-09 (Route: Dashboard)
- AC-01.3.3 → TC-01-10 (IPC Timeout Error UI)
- AC-01.3.4 → TC-01-11 (Route: Invalid Fallback)
- AC-01.4.1, AC-01.4.2 → TC-01-12 (Plugins Init)
- AC-01.4.3 → TC-01-13 (Plugin Panic Handler)
- AC-01.4.4 → TC-01-14 (Log System Degradation)
- AC-01.5.1, AC-01.5.2 → TC-01-15 (State Sync Prep)
- AC-01.5.3 → TC-01-16 (OOM Guard)
- AC-01.5.4 → TC-01-17 (Wait on Hydration)

## C. Test Cases

| TC ID | Scenario | Type | Priority | Preconditions | Test Data | Steps | Expected Result | Coverage |
| -------- | -------------------------------------- | -------- | -------- | ------------------------ | ----------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | -------------------- |
| TC-01-01 | Second instance launch focus | Positive | High | 1 instance running | N/A | 1. Minimize current app window<br>2. Execute`emmm2.exe` again | Existing window focuses in ≤300ms. Second process exits. | AC-01.1.1, AC-01.1.3 |
| TC-01-02 | Fresh install startup | Positive | High | Cleared app data | N/A | 1. Double click executable | App starts. Interactive UI loads in ≤2s. | AC-01.1.2 |
| TC-01-03 | Rapid concurrent double-clicks | Edge | High | N/A | Script: 5 parallel executions | 1. Execute script launching 5 instances instantly | Exactly 1 process survives. No DB locks or crashes. | AC-01.1.4 |
| TC-01-04 | DB creation from scratch | Positive | High |`emmm2.db` missing | N/A | 1. Launch App<br>2. Check AppData folder |`emmm2.db` created. Migration completes ≤500ms. WAL mode enabled. | AC-01.2.1 |
| TC-01-05 | Migrating older DB schema | Positive | High |`schema_v1.db` | Old valid DB injected | 1. Replace DB file with older version<br>2. Launch | Migrates to latest tables incrementally. Data preserved. | AC-01.2.2 |
| TC-01-06 | DB Permission / Read-Only failure | Negative | High | AppData is read-only | N/A | 1. Lock`EMMM2` folder<br>2. Launch | Application halts boot, surfaces native "IO Error" dialog, then exits. | AC-01.2.3, AC-01.2.5 |
| TC-01-07 | Missing App Dir regeneration | Positive | Med | AppData folder deleted | N/A | 1. Delete`EMMM2` root directory<br>2. Launch | The folder is regenerated flawlessly before DB connection begins. | AC-01.2.4 |
| TC-01-08 | Route: Initial setup empty state | Positive | High | DB games == 0 | N/A | 1. Launch fresh DB | Router automatically pushes to`/welcome` immediately post-IPC. | AC-01.3.1 |
| TC-01-09 | Route: Existing configs bypass welcome | Positive | High | DB games >= 1 | Valid Game | 1. Launch | Router bypasses welcome, pushes safely to`/dashboard`. | AC-01.3.2 |
| TC-01-10 | IPC Timeout error boundary | Negative | Med | Force sleep on hook | N/A | 1. Inject 6s sleep in`check_config_status`<br>2. Launch | App shows "Communication Error" UI instead of blocking into a white-screen. | AC-01.3.3 |
| TC-01-11 | Route: Invalid UUID fallback | Edge | Low |`active_game_id` is fake | N/A | 1. Launch with invalid config UUID | App gracefully falls back to the first available ID, doesn't panic. | AC-01.3.4 |
| TC-01-12 | Plugins fully connected to lifecycle | Positive | High | N/A | N/A | 1. Launch<br>2. Query updater status | Plugins (`dialog`,`log`,`updater`) register without error before main mount. | AC-01.4.1 |
| TC-01-13 | Critical Plugin Init failure | Negative | Low | N/A | N/A | 1. Hardcode panic in Tauri plugin setup<br>2. Launch | Exits with OS crash report. No corrupted incomplete state saved in DB. | AC-01.4.3 |
| TC-01-14 | Failed File-logging fallback | Edge | Low |`logs/` directory locked | N/A | 1. Lock log folder<br>2. Launch | Logs execute to memory buffer. App launch sequence does not abort. | AC-01.4.4 |
| TC-01-15 | Global App State Initialization | Positive | High | N/A | N/A | 1. Listen for background events |`SqlitePool`,`WatcherState`,`ScanState` exist securely prior to`run()`. | AC-01.5.1, AC-01.5.2 |
| TC-01-16 | OOM Memory failure at boot | Negative | Low | Capped 10MB container | N/A | 1. Boot | Fails explicitly with logged Out-of-Memory. Process ends. | AC-01.5.3 |
| TC-01-17 | IPC race condition against Locks | Edge | Med | N/A | N/A | 1. Blast 100`get_games` simultaneously at window paint | React waits. Rust Mutex protects hydration. Zero crashes. | AC-01.5.4 |
| TC-01-18 | Configured In-App Restart | Implied | Med |`Settings` | N/A | 1. Trigger App restart inside UI | Old app process natively ends. New PID safely takes over smoothly without Single-Instance deadlocks. | N/A |

## D. Missing / Implied Test Areas

- **Theme Flashing**: Verifying that white-screen flashes do not occur when booting an app configured for Dark Theme.
- **DPI Multi-Monitor Start**: Bounding the newly unminimized instance back to the exact monitor the user is viewing.

## E. Open Questions / Gaps

- _None_

## F. Automation Candidates

- **TC-01-02 (Happy Path Start)**: Critical WebdriverIO E2E checking that DOM mounts within the timeout period.
- **TC-01-04 (DB Fresh Install)**: Rust Unit testing that asserts SQLite file and WAL files effectively exist.
- **TC-01-08, TC-01-09 (Routing)**: Vitest UI testing confirming Router hooks push respectively based directly upon mock`ConfigStatus` return exclusively.
