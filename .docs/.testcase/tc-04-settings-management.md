# Test Cases: Settings Management (req-04)

## A. Requirement Summary

- **Feature Goal**: Provide a centralized, persistent Settings panel for app-wide preferences (Theme, Games, Privacy, Maintenance, Logs) without silent state drift.
- **User Stories**:
 - US-04.1: General Appearance & Behavior
 - US-04.2: Game Configuration Management
 - US-04.3: Privacy & Safe Mode Settings
 - US-04.4: Database Maintenance
 - US-04.5: Factory Reset
 - US-04.6: Error Log Viewer
- **Success Criteria**:
 - Setting update UI response ≤ 100ms.
 - Page load DB hydration ≤ 300ms.
 - Atomic config save ≤ 30ms.
 - Factory reset (wipe + backup) ≤ 3s (50MB DB).
 - DB Optimize maintenance ≤ 10s (10,000 mods).
 - Validated forms (Zod), no DB state desync.
- **Main Risks**: Accidental destructive data wipes (Factory Reset), SQLite locks failing during concurrent maintenance/watcher threads, unhashed PIN storage.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-04-settings-management.md`

- AC-04.1.1, AC-04.1.2 → TC-04-01
- AC-04.1.3 → TC-04-02
- AC-04.1.4 → TC-04-03
- AC-04.2.1, AC-04.2.2 → TC-04-04
- AC-04.2.3 → TC-04-05
- AC-04.2.4 → TC-04-06
- AC-04.2.5 → TC-04-07
- AC-04.2.6 → TC-04-08
- AC-04.2.7 → TC-04-09
- AC-04.3.1, AC-04.3.2 → TC-04-10
- AC-04.3.3 → TC-04-11
- AC-04.3.4 → TC-04-12
- AC-04.4.1, AC-04.4.2 → TC-04-13
- AC-04.4.3 → TC-04-14
- AC-04.4.4 → TC-04-15
- AC-04.4.5 → TC-04-16
- AC-04.5.1, AC-04.5.2 → TC-04-17
- AC-04.5.3 → TC-04-18
- AC-04.5.4 → TC-04-19
- AC-04.5.5 → TC-04-20
- AC-04.6.1, AC-04.6.2 → TC-04-21
- AC-04.6.3 → TC-04-22
- AC-04.6.4 → TC-04-23

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ------------------------------- | -------- | -------- | ------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ---------------- | -------------------- |
| TC-04-01 | Toggle Theme & Auto-Close | Positive | High | Theme`dark` | 1. Open Settings > General.<br>2. Select 'Dark' theme.<br>3. Toggle 'Auto-Close'.<br>4. Restart app. | Visuals update in ≤ 100ms. Values persist upon restart. | S2 | AC-04.1.1, AC-04.1.2 |
| TC-04-02 | Unsupported language protection | Negative | Low | Language`zh-CN` | 1. Open Settings > General.<br>2. Select non-English language in dropdown. | Tooltip shows "Only English supported". Input reverts automatically. | S4 | AC-04.1.3 |
| TC-04-03 | Rapid toggling race state | Edge | Med | Rapid clicks | 1. Rapidly click Theme dropdown 10 times in 1s. | Optimistic state handles rapid input. Final DB save matches the exact last selection. | S3 | AC-04.1.4 |
| TC-04-04 | Add Game via Settings | Positive | High | Valid Genshin folder | 1. Settings > Games.<br>2. Click "Add Game".<br>3. Fill path and submit. | Game populates on the list inside the tab instantly without a page reload (≤ 300ms). | S2 | AC-04.2.1, AC-04.2.2 |
| TC-04-05 | Rescan specific game | Positive | Med | Valid game entry | 1. Settings > Games.<br>2. Click "Rescan Library" on a game row. | Progress indicator opens. Scan operates explicitly scoped to that specific game. | S2 | AC-04.2.3 |
| TC-04-06 | Delete inactive game | Positive | High | 2 games setup | 1. Have active Game A, inactive Game B.<br>2. Click Remove on Game B.<br>3. Confirm. | Game B vanishes from the DB and UI list in ≤ 300ms. Game A remains active. | S2 | AC-04.2.4 |
| TC-04-07 | Delete active game resets app | Negative | High | 1 active game | 1. Remove the active game from Settings.<br>2. Confirm. | Active game ID clears. Router strictly forces navigation to`/welcome`. | S1 | AC-04.2.5 |
| TC-04-08 | Duplicate game block | Negative | Med | Existing path | 1. Try to add a game with the exact path and type of an existing game. | Zod UI / Backend block the creation with "Duplicate" error. No DB record added. | S3 | AC-04.2.6 |
| TC-04-09 | Form path physical validation | Negative | High | Fake path`X:\Fake` | 1. Enter fake path in Add Game modal.<br>2. Submit. | Inline input turns red with validation error. Rust backend physically checks`Path::new(path).exists()` and rejects. | S2 | AC-04.2.7 |
| TC-04-10 | Save Privacy PIN & Keywords | Positive | High | PIN`1234` | 1. Settings > Privacy.<br>2. Enter PIN and save.<br>3. Add 'nsfw' to keywords and save. | PIN hashes to Argon2 (no plaintext in`config.json`). Keywords save and. | S1 | AC-04.3.1, AC-04.3.2 |
| TC-04-11 | PIN Lockout 5 Failures | Negative | High | Incorrect PINs | 1. Attempt invalid PIN 5 times rapidly. | Field disables. Countdown timer "Try again in 60s" blocks further brute force. | S1 | AC-04.3.3 |
| TC-04-12 | Force Exclusive Mode pinning | Edge | Med |`force_exclusive = true` | 1. Set`force_exclusive_mode` to true.<br>2. Check normal UI toggle. | Normal Safe Mode toggle in the main UI is locked to ON. Cannot be disabled without Settings. | S2 | AC-04.3.4 |
| TC-04-13 | Manual DB Maintenance | Positive | High | Stale garbage thumbs | 1. Settings > Advanced.<br>2. Click "Run Maintenance". | Executes SQLite optimize, purges orphans, and deletes empty trash. Finishes <10s. Success toast shows byte/row counts. | S2 | AC-04.4.1, AC-04.4.2 |
| TC-04-14 | Weekly Maintenance Scheduler | Positive | Med | 35-day old trash | 1. Fast-forward OS clock/Tokio interval by 7 days.<br>2. Observe background logs. | Runs silently in background task. Cleans 30-day old thumbnails/trash automatically. | S2 | AC-04.4.3 |
| TC-04-15 | Maintenance vs OperationLock | Negative | High | Active Mod Toggle | 1. Trigger heavy 100-mod toggle.<br>2. Instantly click "Run Maintenance". | Maintenance aborted with "Database busy". UI thread does not freeze or panic. | S1 | AC-04.4.4 |
| TC-04-16 | Maintenance mid-crash survival | Edge | High | Mid-optimize | 1. Start maintenance.<br>2. Kill process violently in task manager.<br>3. Boot app. | SQLite WAL recovers DB. No table corruption. | S1 | AC-04.4.5 |
| TC-04-17 | Factory Reset full wipe | Positive | Critical | Data filled app | 1. Settings > Advanced.<br>2. Click "Reset App".<br>3. Type "RESET".<br>4. Submit. | Backend creates`*.bak` timestamped backup, wipes tables, clears Zustand/localStorage, routes to`/welcome`. | S1 | AC-04.5.1, AC-04.5.2 |
| TC-04-18 | Factory Reset Backup IO failure | Negative | High | Locked`backups/` dir | 1. Lock backup directory via OS.<br>2. Trigger Reset. | Backup creation fails. Wipe immediately aborted. Explicit error shown. Prior data safely survives. | S1 | AC-04.5.3 |
| TC-04-19 | Factory Reset typo dismiss | Negative | Med | Text`RES` | 1. Click Factory Reset.<br>2. Type "RES".<br>3. Hit Enter/Submit. | Form disabled. No action occurs. | S3 | AC-04.5.4 |
| TC-04-20 | Factory Reset vs Watcher | Edge | Critical | Active folder sync | 1. Drop files into Game directory.<br>2. Trigger Factory Reset simultaneously. |`WatcherState` explicitly stopped first before DB tables are dropped. Stops ghost events from repopulating wiped tables. | S1 | AC-04.5.5 |
| TC-04-21 | View Error Logs | Positive | Med | Known Log generated | 1. Open Settings > Logs.<br>2. View entries. | List populates instantly from`tauri-plugin-log`. Timestamps and badges exist. | S3 | AC-04.6.1 |
| TC-04-22 | Filter Logs | Positive | Med | Multi-level logs | 1. Select 'ERROR' in dropdown. | Excludes INFO/WARN lines in UI. | S3 | AC-04.6.2 |
| TC-04-23 | Huge Log Truncation | Edge | Med | 100MB log file | 1. Generate massive log file physically.<br>2. Open Settings > Logs. | App smoothly reads only the tail 500 lines. No OOM crashing on reading file. | S2 | AC-04.6.4 |

## D. Missing / Implied Test Areas

- **Backend`SettingKey` restriction**: Injecting a fake key via IPC to`set_preference` should be safely trapped by the enum serialization.
- **Missing`tauri-plugin-log`**: If the plugin fails to init (no folder perms), does the Log tab crash the UI or show gracefully empty? Requirement AC-04.6.3 mentions unreadable file handling.

## E. Open Questions / Gaps

- **Backup accumulate**: Does Factory Reset accumulate backups infinitely in`backups/`, or does it prune them (e.g., keep last 5)?

## F. Automation Candidates

- **TC-04-01 (General Toggles)**: Cypress/Playwright checking`data-theme` HTML attribute injection.
- **TC-04-17 (Factory Reset logic)**: E2E testing the validation disabled state of the input block enforcing typed`"RESET"`.
- **TC-04-13 (Maintenance SQLx)**: Rust unit test confirming DB file byte sizes decrease.

## G. Test Environment Setup

- **Preconditions**: Dev build populated with 2 active configured games, a`tauri-plugin-log` artifact generated, and dummy preference flags set.
- **Database State**: At least 5 dead thumbnail image files generated physically without DB associations to ensure Maintenance prunes them.

## H. Cross-Epic E2E Scenarios

- **E2E-04-01 (Wipe and Remount)**: Populate game (Epic 02), fill 100 mods (Epic 13), execute Factory Reset (Epic 04), verify Router bounces to Welcome (Epic 03) and all tables are dropped, ensuring a clean slate.
