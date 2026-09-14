# Epic 04: Settings Management

## 1. Executive Summary

- **Problem Statement**: Users need persistent control over app-wide preferences (theme, game paths, data hygiene, error monitoring) in a single place — without re-configuring after restarts or dealing with silent state drift between the UI and backend.
- **Proposed Solution**: A centralized Settings panel (tabs: General / Games / Privacy / Advanced / Logs) backed by a DB key-value store, with optimistic preference updates, game CRUD proxy, a "Rescan Library" trigger, a weekly scheduled maintenance task (orphan cleanup + thumbnail LRU), a guarded factory reset with pre-wipe backup, and an Error Log Viewer reading from `tauri-plugin-log` files.
- **Success Criteria**:
  - App reaches interactive state (window fully rendered, IPC ready) in ≤ 2s on an NTFS SSD from cold start.
  - Multi-language localization (EN, ID, ZH) initializes in ≤ 200ms during boot.
  - DB migration completes in ≤ 500ms for a database with up to 10,000 rows.
  - Atomic config save completes in ≤ 30ms (`*.tmp` → rename).
  - Factory reset completes (wipe + backup) in ≤ 3s for a DB up to 50MB.
  - Database maintenance completes in ≤ 10s for a DB with 10,000 mod records.
  - 0 cases of preference state desync between Zustand store and DB after toggle — verified by E2E test.
  - All settings forms validated with `React Hook Form + Zod` — invalid paths show inline red errors; Save is disabled until valid.

---

## 2. User Experience & Functionality

### User Stories

#### US-04.1: General Appearance & Behavior

As a user, I want to configure the app's theme and behavior from the General tab, so that the app matches my preferences and saves across restarts.

| ID        | Type        | Criteria                                                                                                                                                                                                                            |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-04.1.1 | ✅ Positive | Given the General tab, when I change Theme (Dark / Light / System / Cyberpunk / Onyx), then the CSS theme class changes on `<html>` within ≤ 100ms and the value persists across restart; UI must adhere to **Zero-Literal Policy** |
| AC-04.1.2 | ✅ Positive | Given the General tab, when I toggle "Auto-Close on Launch", then the setting is persisted to DB and the next game launch respects the new value                                                                                    |
| AC-04.1.3 | ✅ Positive | Given the Language dropdown, when I select English (EN), Indonesian (ID), or Chinese (ZH), then the UI swaps languages instantly (≤ 100ms) and persists the choice                                                                  |
| AC-04.1.4 | ⚠️ Edge     | Given a language selection not yet supported, then a tooltip "Requested language not yet available" is shown and the selection is reverted                                                                                          |

---

#### US-04.2: Game Configuration Management

As a user, I want to manage my configured games from the Settings → Games tab, so that I can add games, remove uninstalled ones, or trigger a rescan without leaving settings.

| ID        | Type        | Criteria                                                                                                                                                                                                 |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-04.2.1 | ✅ Positive | Given the Games settings tab, then I see each configured game's name, type badge, root path, and a remove button                                                                                         |
| AC-04.2.2 | ✅ Positive | Given I click "Add Game", then the same manual-add modal used in onboarding opens; on submit, the new game appears in the list without page reload                                                       |
| AC-04.2.3 | ✅ Positive | Given I click "Rescan Library" for a game, then `trigger_scan(game_id)` (Epic 25) fires and a progress indicator appears — duplicate to the onboarding scan trigger but scoped to that game              |
| AC-04.2.4 | ✅ Positive | Given I click the remove icon on a non-active game, then it is deleted from DB and removed from the list in ≤ 300ms                                                                                      |
| AC-04.2.5 | ❌ Negative | Given I remove the active game, then `active_game_id` is cleared and the app immediately navigates to `/welcome`                                                                                         |
| AC-04.2.6 | ❌ Negative | Given I attempt to add a game with the same `game_type` + `root_path` as an existing entry, then "Game already configured" error is shown — no duplicate DB record created                               |
| AC-04.2.7 | ⚠️ Edge     | Given a game path is changed to an invalid location and saved, then Zod validation turns the input red before submit — the backend additionally verifies `Path::new(path).exists()` and rejects if false |

---

#### US-04.3: Privacy & Safe Mode Settings

As a user, I want to configure Safe Mode keywords and PIN security from the Privacy tab, so that my sensitive content is properly gated.

| ID        | Type        | Criteria                                                                                                                                                                               |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-04.3.1 | ✅ Positive | Given I set a PIN, then it is stored as a hashed value (Argon2) in `config.json` — the raw PIN is never persisted                                                                      |
| AC-04.3.2 | ✅ Positive | Given the keyword list (e.g., "naked", "nsfw"), when I add/remove keywords, then they are saved to `safe_mode.keywords: Vec<String>` and applied on the next Safe Mode activation      |
| AC-04.3.3 | ❌ Negative | Given 5 consecutive wrong PIN attempts, then the PIN input is blocked for 60s; a countdown timer shows "Try again in {n}s"                                                             |
| AC-04.3.4 | ⚠️ Edge     | Given `force_exclusive_mode = true` (Safe Mode always ON), then the Safe Mode toggle on the UI is locked to ON — the user must disable `force_exclusive_mode` in Settings to toggle it |

---

#### US-04.4: Database Maintenance

As a user, I want to run DB cleanup from the Advanced tab, so that orphaned thumbnails and stale records are purged and disk space is freed.

| ID        | Type        | Criteria                                                                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-04.4.1 | ✅ Positive | Given I click "Run Maintenance", then the backend runs `PRAGMA optimize`, prunes thumbnails with no matching folder record, deletes empty trash folders, and returns a summary (rows deleted, bytes freed) in ≤ 10s                             |
| AC-04.4.2 | ✅ Positive | Given maintenance completes, then a success toast shows exactly how many thumbnails and orphan rows were removed                                                                                                                                |
| AC-04.4.3 | ✅ Positive | Given the weekly scheduled maintenance task fires (Tokio interval), then it automatically: removes orphaned `collection_items`, deletes stale thumbnails (last accessed > 30 days), purges empty trash entries — results logged at `info` level |
| AC-04.4.4 | ❌ Negative | Given the DB is locked by an active scan or bulk operation (`OperationLock` held), when I click maintenance, then it returns "Database busy — please wait" without blocking the UI thread                                                       |
| AC-04.4.5 | ⚠️ Edge     | Given the app crashes mid-maintenance, then on next launch the SQLite WAL journal rolls back any incomplete transaction — no table corruption                                                                                                   |

---

#### US-04.5: Factory Reset

As a user, I want a guarded option to completely reset the app's data without losing raw mod files on disk, so that I can recover from a corrupted state.

| ID        | Type        | Criteria                                                                                                                                                                        |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-04.5.1 | ✅ Positive | Given I click "Reset App" and confirm by typing "RESET", then the backend creates a timestamped backup of `.db` in `app_data_dir/backups/`, wipes all tables, completes in ≤ 3s |
| AC-04.5.2 | ✅ Positive | Given the DB wipe completes, then the frontend clears Zustand state + `localStorage` and hard-navigates to `/welcome`                                                           |
| AC-04.5.3 | ❌ Negative | Given a filesystem permissions error prevents DB backup creation, then the reset is aborted, a clear error dialog is shown, and no data is deleted                              |
| AC-04.5.4 | ❌ Negative | Given the user dismisses the confirmation dialog without typing "RESET", then no data is changed                                                                                |
| AC-04.5.5 | ⚠️ Edge     | Given a factory reset fires while the file watcher is active, then `WatcherState` is explicitly stopped before any DB tables are dropped — preventing stale events during reset |

---

#### US-04.6: Error Log Viewer

As a developer/power-user, I want to view recent application logs from within Settings, so that I can diagnose issues without navigating to obscure log files.

| ID        | Type        | Criteria                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-04.6.1 | ✅ Positive | Given Settings > Logs tab, when opened, then recent log entries from `tauri-plugin-log` files are displayed in a scrollable list with timestamp, level badge (INFO / WARN / ERROR), and message |
| AC-04.6.2 | ✅ Positive | Given a level filter dropdown (All / WARN / ERROR), when I select ERROR, then only `ERROR` level entries are shown — INFO and WARN are hidden                                                   |
| AC-04.6.3 | ❌ Negative | Given the log file doesn't exist or is unreadable (fresh install), then the Logs tab shows "No logs found" — no crash or empty white screen                                                     |
| AC-04.6.4 | ⚠️ Edge     | Given the log file exceeds 10MB, then only the last 500 lines are loaded — no OOM from reading the entire file into memory                                                                      |

---

### Non-Goals

- No per-game settings; all settings are global, shared across all configured games.
- No cloud backup or sync of preferences to any external service.
- No import/export of settings file via UI.
- No user accounts or per-profile settings; single-user local install.
- Log Viewer is read-only — no in-app log deletion or level change.

---

## 3. Technical Specifications

### Architecture Overview

```
Settings Page (React)
  ├── Tab: General
  │   ├── theme toggle → commands.setPreference({ key: 'theme', value })
  │   └── auto_close toggle → commands.setPreference({ key: 'auto_close', value })
  ├── Tab: Games
  │   ├── list → useQuery('games', get_games)
  │   ├── add → AddGameModal (Epic 02)
  │   ├── rescan → commands.triggerScan({ game_id })
  │   └── remove → save_settings({ games: games_without_removed_entry })
  ├── Tab: Privacy
  │   ├── PIN set/change → commands.setPin({ plain_pin }) → Argon2 hash stored
  │   ├── keywords list → commands.setSafeModeKeywords({ keywords })
  │   └── force_exclusive_mode toggle
  ├── Tab: Advanced
  │   ├── maintenance → commands.runDbMaintenance() → MaintenanceResult
  │   └── factory reset → commands.factoryReset() → navigate('/welcome')
  └── Tab: Logs
      └── commands.getLogs({ limit: 500, count: 500 }) → Vec<String>

Backend
  ├── set_preference(key: SettingKey, value) → preferences KV table
  ├── run_db_maintenance() → { rows_deleted, bytes_freed }
  ├── factory_reset() → backup DB → wipe tables → seed empty migrations
  ├── set_pin(plain_pin) → Argon2::hash(plain_pin) → save to config
  └── get_logs(limit, count) → read tauri-plugin-log file tail using app_log_dir()

Weekly Scheduled Maintenance (Tokio interval, 7 days):
  remove orphaned collection_items WHERE folder_id NOT IN (SELECT id FROM folders)
  delete thumbnails WHERE last_accessed < NOW() - 30 days
  purge empty trash entries WHERE date_trashed < NOW() - 30 days
```

### Integration Points

| Component        | Detail                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Persistence      | SQLite `app_settings` (K-V) and `games` tables; `ConfigService` uses `Mutex<AppSettings>` and synchronous DB writes                         |
| PIN Hashing      | `Argon2` via `argon2` crate; stored in `safe_mode.pin_hash`                                                                                 |
| Mode Switch      | `PrivacyManager::switch_mode` → atomic database updates + corridor directory moves + watcher suppression                                    |
| Maintenance      | `maintenance_service.rs` → `VACUUM`, `ThumbnailCache::prune_orphans`, `remove_orphaned_collection_items`, `cleanup_old_empty_trash_entries` |
| Trash Purge      | `cleanup_old_empty_trash_entries` logic in `maintenance_service.rs` handles folders > 30 days old                                           |
| Log Viewer       | `tauri-plugin-log` integration via `SettingsPage` log tab                                                                                   |
| Validation       | Frontend: `react-hook-form` + `zod`; Backend: `normalize_keywords` trim/lowercase                                                           |
| Game Setup       | `commands.addGame()` and `commands.autoDetectGames()` (Epic 03)                                                                             |
| Weekly Scheduler | `tokio::time::interval(Duration::from_secs(7 * 86400))` — runs in background task                                                           |

### Security & Privacy

- **Factory reset requires typed "RESET" confirmation** — not just a checkbox — to prevent accidental data loss.
- **DB backup is written before any destructive operation** — if backup write fails, reset is aborted entirely.
- **`set_preference` validates key against `SettingKey` enum allowlist** — arbitrary key injection rejected.
- **Argon2 PIN** is never stored in plain text; `pin_hash: Option<String>` in `SafeModeConfig`.
- **Log Viewer is read-only** — no user input is written to log files; no injection surface.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — DB pool, preferences table), Epic 02 (Game Management — `add_game`, settings-backed game list persistence).
- **Blocks**: Nothing directly — settings are consumed by most other epics (active game, theme, auto-close, safe mode config).
