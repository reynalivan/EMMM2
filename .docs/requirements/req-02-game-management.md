# Epic 02: Game Management

## 1. Executive Summary

- **Problem Statement**: Users manage mods for multiple games (Genshin, HSR, ZZZ, WuWa, Endfield) from the same tool, but adding those games manually is error-prone and the app has no way to auto-discover them — leading to misconfigured paths and broken mod loading.
- **Proposed Solution**: A game management system with auto-detection heuristics, path-validated manual addition, safe removal with state cleanup, a sequenced launch pipeline (loader → game exe), and reactive game switching that restarts the file watcher and re-fetches all queries.
- **Success Criteria**:
  - Auto-detect completes in ≤ 1s when scanning up to 5 drive roots simultaneously.
  - Manual game addition completes in ≤ 300ms from submit to DB record confirmed.
  - Game launch spawns the mod loader process in ≤ 200ms after the user clicks launch.
  - Active game switch propagates to all frontend queries (objectlist, grid, preview) in ≤ 200ms.
  - Zero duplicate game records in the `games` table across 100 rapid concurrent add attempts.

---

## 2. User Experience & Functionality

### User Stories

#### US-02.1: Auto-Detect Games

As a user, I want the app to auto-detect installed games from my launcher folder, so that I can set up quickly without manually entering paths.

| ID        | Type        | Criteria                                                                                                                                                                                                  |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-02.1.1 | ✅ Positive | Given a valid root path containing recognized game subfolders, when auto-detect is invoked, then each valid subfolder is parsed into a game config and saved to the DB — completing in ≤ 1s for ≤ 5 games |
| AC-02.1.2 | ✅ Positive | Given a detected game, then its `mods_path`, `game_exe`, and `loader_exe` are derived. The validation strictly requires `Mods/` folder, `d3dx.ini`, `d3d11.dll` and an exe containing "loader"            |
| AC-02.1.3 | ❌ Negative | Given a root path that contains no valid game subfolders, when auto-detect runs, then an empty list is returned without triggering an error dialog                                                        |
| AC-02.1.4 | ❌ Negative | Given a root path that does not exist or lacks read permissions, when auto-detect runs, then a clear validation error is surfaced to the UI — not a Rust panic                                            |
| AC-02.1.5 | ⚠️ Edge     | Given a folder with symlink loops or maliciously deep nesting (> 5 levels), when auto-detect scans it, then the scanner bails out cleanly with a depth-limit guard — no infinite recursion                |

---

#### US-02.2: Manual Game Addition

As a user, I want to manually add a game by selecting its type and folder path, so that I can configure games the auto-detect heuristic missed.

| ID        | Type        | Criteria                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-02.2.1 | ✅ Positive | Given a valid `SupportedGame` variant and a valid folder path, when submission is invoked, then the game is validated, assigned a UUID, and persisted to `games` table in ≤ 300ms                 |
| AC-02.2.2 | ✅ Positive | Given the newly added game is the first game in the DB, then it is automatically set as the `active_game_id` and the frontend re-routes to `/dashboard`                                           |
| AC-02.2.3 | ❌ Negative | Given a game type + physical path combination already exists in the DB, when manual addition is invoked, then a `DuplicateGame` error is returned — no second record is created                   |
| AC-02.2.4 | ❌ Negative | Given a path missing the `/Mods` folder, `d3dx.ini`, `d3d11.dll`, or a loader `.exe`, when add is invoked, then the backend returns a `PathValidationError` showing which core file is missing    |
| AC-02.2.5 | ❌ Negative | Given an unrecognized or spoofed `game_type` string in the RPC payload, when the backend deserializes it, then `serde` fails safely and returns a typed error — never executes unknown code paths |
| AC-02.2.6 | ⚠️ Edge     | Given multiple rapid clicks on the "Submit" button before the first request resolves, then the backend `INSERT` is idempotent (UNIQUE constraint) and only one DB record is created               |

---

#### US-02.3: Remove Game

As a user, I want to remove a game configuration, so that I can clean up games I no longer play without leaving orphan data.

| ID        | Type        | Criteria                                                                                                                                                                                            |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-02.3.1 | ✅ Positive | Given a valid `game_id`, when remove is invoked, then the game row is deleted from the `games` table and all associated DB records (objects, mods) are cascade-deleted                              |
| AC-02.3.2 | ❌ Negative | Given the removed game was the last one, when removal completes, then `active_game_id` is cleared and the app re-routes to `/welcome`                                                               |
| AC-02.3.3 | ❌ Negative | Given a non-existent or already-removed `game_id`, when remove is invoked, then the backend returns a `NotFound` error and no data changes occur                                                    |
| AC-02.3.4 | ⚠️ Edge     | Given the game being removed is currently being scanned (Scan Engine running), when remove is triggered, then the scanner is cancelled and all file locks are released before the DB row is deleted |

---

#### US-02.4: Launch Game

As a user, I want to launch my game with mod injections active, so that I can play with my current mod setup without manual steps.

| ID        | Type        | Criteria                                                                                                                                                                                                    |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-02.4.1 | ✅ Positive | Given a configured game with a loader exe, when launch is invoked, then the mod loader starts first, followed by the game exe — both as detached child processes — with the loader launching within ≤ 200ms |
| AC-02.4.2 | ✅ Positive | Given `auto_close_on_launch = true`, when the game launches successfully, then the EMMM2 process terminates gracefully within 2s                                                                            |
| AC-02.4.3 | ✅ Positive | Given custom launch arguments are stored for the game, when launching, then the arguments are appended verbatim to the game exe process                                                                     |
| AC-02.4.4 | ❌ Negative | Given no game config found for the given `game_id`, when launch is invoked, then a "Game not found" error toast is shown — no process is spawned                                                            |
| AC-02.4.5 | ❌ Negative | Given the game executable path no longer exists on disk, when launch is invoked, then an `IO: NotFound` error is returned to the user — no crash                                                            |
| AC-02.4.6 | ⚠️ Edge     | Given the mod loader hangs waiting for UAC elevation, the launch sequence does not block the entire UI thread — the process spawn is done in an async Tokio task                                            |

---

#### US-02.5: Active Game Switching

As a user, I want to switch between configured games from the top bar, so that I can manage mods for different games in the same session.

| ID        | Type        | Criteria                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-02.5.1 | ✅ Positive | Given multiple configured games, when switching the active game, then the new `game_id` is persisted to preferences and the `activeGameId` Zustand store updates in ≤ 200ms |
| AC-02.5.2 | ✅ Positive | Given an active game change, then the file watcher restarts on the new `mods_path` and all React Query caches (`['folders']`, `['objects']`) are invalidated immediately    |
| AC-02.5.3 | ❌ Negative | Given `null` or an invalid `game_id` submitted as the active target, then the frontend receives a validation error and the active game state is not changed                 |
| AC-02.5.4 | ⚠️ Edge     | Given an active game switch triggered while a bulk operation is in progress for the old game, the `OperationLock` is awaited before the watcher restarts on the new path    |

---

### Non-Goals

- No support for games outside the predefined `SupportedGame` enum without a code change and app update.
- No remote game library sync (Steam, Epic, etc.) — paths are always local filesystem only.
- No game version management or game file patching.
- No Linux/macOS support in this phase — Windows (NTFS) only.
- Removing a game does NOT delete any mod files on disk — only DB records.

---

## 3. Technical Specifications

### Architecture Overview

```
commands/games/
  ├── auto_detect_games(root_path)  → Vec<GameConfig>
  ├── add_game(game_type, path)     → GameRecord
  ├── remove_game(game_id)          → ()
  ├── launch_game(game_id)          → ()
  └── set_active_game(game_id)      → ()

services/games/
  ├── detector.rs   — heuristic subfolder scanner
  ├── validator.rs  — path + exe existence checks
  └── launcher.rs   — detached process spawner (Tokio::process::Command)

DB tables: games(id, game_type, name, root_path, mods_path, game_exe, loader_exe, launch_args, created_at)
```

### Integration Points

| Component      | Detail                                                                           |
| -------------- | -------------------------------------------------------------------------------- |
| DB             | `games` table — primary key `UUID`, `UNIQUE(game_type, root_path)` constraint    |
| File Watcher   | `WatcherState` restarted on `set_active_game` via `init_watcher(new_mods_path)`  |
| Frontend State | `useAppStore.activeGameId` + `useGames` React Query invalidated on all mutations |
| Process Spawn  | `tokio::process::Command` — `spawn()` (detached, no `wait()`)                    |
| Path Dialog    | `tauri-plugin-dialog` — `open({ directory: true })`                              |

### Security & Privacy

- **`mods_path` and all stored paths are validated with `std::fs::canonicalize()`** before DB write — rejects symlinks pointing outside the given root and non-existent paths.
- **Game exe path is never passed to `shell_execute` or `cmd.exe`** — only to `std::process::Command::new(path)` with explicit argument arrays, preventing shell injection.
- **No network calls** during game management operations; detection and validation are purely local.
- **Launch arguments** stored per-game must be treated as untrusted user input — no shell interpolation; they are passed as a `Vec<String>` to `args()`, never as a single concatenated string.
- **Auto-detect depth limit**: scanner stops at 5 levels deep regardless of symlinks to prevent DoS via crafted directory structures.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap) — DB pool must be initialized before any game CRUD.
- **Blocks**: Epic 03 (Onboarding), Epic 05 (Workspace Layout), Epic 09 (Object Schema), Epic 28 (File Watcher), Epic 34 (App Updater) — all depend on a valid active game context.
