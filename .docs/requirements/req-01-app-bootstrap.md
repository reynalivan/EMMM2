# Epic 01: App Bootstrap & Initialization

## 1. Executive Summary

- **Problem Statement**: Without a structured startup pipeline, concurrent launches corrupt the database, missing migrations crash commands, and users land on the wrong screen — making the app unreliable from the very first interaction.
- **Proposed Solution**: A sequenced boot pipeline that enforces single-instance, runs DB migrations atomically, registers all global state before the window shows, and routes to the correct initial screen based on config presence.
- **Success Criteria**:
  - App reaches interactive state (window fully rendered, IPC ready) in ≤ 2s on an NTFS SSD from cold start.
  - DB migration completes in ≤ 500ms for a database with up to 10,000 rows.
  - Second-instance focus round-trip (detect → focus existing window) completes in ≤ 300ms.
  - Zero unhandled panics during the boot sequence in the last 30 days of production telemetry.
  - Config-status routing resolves to the correct initial route in 100% of test cases (fresh install → `/welcome`, existing config → `/dashboard`).

---

## 2. User Experience & Functionality

### User Stories

#### US-01.1: Single Instance Guard

As a user, I want the app to prevent launching multiple instances, so that I avoid data corruption and conflicting file operations.

| ID        | Type        | Criteria                                                                                                                                                                           |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-01.1.1 | ✅ Positive | Given the app is already running, when the user launches a second instance, then the existing window is focused and unminimized within 300ms                                       |
| AC-01.1.2 | ✅ Positive | Given no other instance is running, when the user launches the app, then it starts normally without any single-instance interference                                               |
| AC-01.1.3 | ❌ Negative | Given the app is running, when a second instance tries to start, then it does NOT create a new window — it silently exits after focusing the existing one                          |
| AC-01.1.4 | ⚠️ Edge     | Given rapid concurrent launches of the executable (e.g., double-click race), then the instance lock resolves cleanly and exactly one window survives without thrashing or deadlock |

---

#### US-01.2: Database Initialization

As a system, I want the local SQLite database to be created and migrated on startup, so that all tables exist and are schema-correct before any command is invoked.

| ID        | Type        | Criteria                                                                                                                                                                                |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-01.2.1 | ✅ Positive | Given a fresh install (no DB file), when the app starts, then the file is created in `app_data_dir/emmm2.db` with WAL mode enabled and all migration tables applied in ≤ 500ms          |
| AC-01.2.2 | ✅ Positive | Given an existing database at an older schema version, when the app starts, then all pending SQLx migrations are applied incrementally without data loss                                |
| AC-01.2.3 | ❌ Negative | Given a migration failure (e.g., locked DB or corrupted schema), when the app starts, then the process displays a clear native error dialog and exits cleanly — no partial boot state   |
| AC-01.2.4 | ❌ Negative | Given the `app_data_dir` does not exist, when the app starts, then it is created automatically before the DB connection attempt — not after a crash                                     |
| AC-01.2.5 | ⚠️ Edge     | Given filesystem permission errors (e.g., read-only volume) preventing DB creation, then the app catches the `IO Error` and surfaces a user-facing message rather than silently hanging |
| AC-01.2.6 | ✅ Positive | Given a fresh install but a legacy `config.json` exists, when the DB is created, it automatically migrates the settings and games from `config.json` into SQLite before proceeding      |

---

#### US-01.3: Config Status Routing

As a user, I want the app to detect whether I have games configured, so that I am routed to the correct initial screen without manual navigation.

| ID        | Type        | Criteria                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-01.3.1 | ✅ Positive | Given zero configured games in the database (`FreshInstall` status), when the app loads, then the frontend is redirected to `/welcome` within 200ms of IPC resolution                         |
| AC-01.3.2 | ✅ Positive | Given ≥ 1 configured games (`HasConfig` status), when the app loads, then the frontend is redirected to `/dashboard`                                                                          |
| AC-01.3.3 | ❌ Negative | Given the `check_config_status` command times out (> 5s) or returns an error, when the app loads, then it displays a clear communication error overlay — not a blank screen                   |
| AC-01.3.4 | ⚠️ Edge     | Given the stored active `game_id` is deleted externally (e.g., direct DB edit), when the app loads, then it falls back to the first available game or redirects to `/welcome` — never crashes |

---

#### US-01.4: Plugin Registration

As a system, I want all native Tauri plugins to be registered at startup, so that downstream features (logging, dialogs, process management, updater) are available when first invoked.

| ID        | Type        | Criteria                                                                                                                                                                                   |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-01.4.1 | ✅ Positive | Given app startup, then all required plugins (`dialog`, `single_instance`, `opener`, `process`, `log`, `updater`) are initialized before the window is shown                               |
| AC-01.4.2 | ✅ Positive | Given the `single_instance` plugin is active, when a second instance is launched, then the callback correctly invokes `window.set_focus()` and `window.unminimize()` on the primary window |
| AC-01.4.3 | ❌ Negative | Given any plugin fails to initialize, then the application handles the panic, terminates cleanly, and prevents a partial boot state from persisting                                        |
| AC-01.4.4 | ⚠️ Edge     | Given the OS restricts log file write permissions, then the logging subsystem degrades to memory-only mode without halting the entire boot sequence                                        |

---

#### US-01.5: Managed State Registration

As a system, I want all global state objects (`ScanState`, `DedupState`, `WatcherState`, `OperationLock`, DB pool) to be registered as Tauri managed state before the window shows, so that commands never access uninitialized state.

| ID        | Type        | Criteria                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-01.5.1 | ✅ Positive | Given app startup, the builder registers: DB pool, `Arc<Mutex<ScanState>>`, `Arc<Mutex<DedupState>>`, `WatcherState`, and `OperationLock` — all before `run()` is called    |
| AC-01.5.2 | ✅ Positive | Given the async DB initialization, the connection pool is registered as managed state in ≤ 500ms before the window becomes visible                                          |
| AC-01.5.3 | ❌ Negative | Given memory allocation fails during state registration, the app exits with an out-of-memory error report logged to disk — not a silent hang                                |
| AC-01.5.4 | ⚠️ Edge     | Given concurrent IPC command invocations arrive exactly as the window shows (race window), state `Mutex` locks block callers until fully hydrated — no `None` unwrap panics |

---

### Non-Goals

- No in-memory pre-loading or caching of mod folder data during bootstrap; that is deferred to first navigation.
- No splash screen or animated loading branding during startup.
- No telemetry or crash-reporting to external servers; logs are written to local `app_data_dir/logs/` only.
- No per-user account system or remote authentication during bootstrap.

---

## 3. Technical Specifications

### Architecture Overview

```
main.rs
  ├── tauri::Builder::default()
  │   ├── plugin: single_instance (callback: focus + unminimize)
  │   ├── plugin: log (rolling file sink)
  │   ├── plugin: dialog, opener, process, updater
  │   └── setup closure:
  │       ├── db::init_pool(app_data_dir) → SqlitePool (migrations auto-applied)
  │       ├── services::images::init_thumbnail_cache(app_data_dir)
  │       ├── manage: SqlitePool, ScanState, DedupState, WatcherState, OperationLock
  │       └── return Ok(())
  └── .run() → window shows

Frontend App.tsx
  ├── mount: init log interceptor
  ├── invoke('check_config_status') → FreshInstall | HasConfig
  ├── route → /welcome  (FreshInstall)
  │       → /dashboard (HasConfig)
  └── background: useAppUpdater.check()
```

**Boot sequence is strictly sequential**: plugins → DB → state → window. No parallelism across these phases to avoid race conditions.

### Integration Points

| Component       | Detail                                                                                 |
| --------------- | -------------------------------------------------------------------------------------- |
| DB Init         | `sqlx::SqlitePool`, migrations in `migrations/` folder via `sqlx::migrate!()` macro    |
| Single Instance | `tauri-plugin-single-instance` v2                                                      |
| Logging         | `tauri-plugin-log` v2 — rolling files in `app_data_dir/logs/`                          |
| Updater         | `tauri-plugin-updater` v2 — silent background check on mount                           |
| Config Status   | `commands/app/settings_cmds.rs` → `check_config_status` — `SELECT COUNT(*) FROM games` |
| Thumbnail Cache | `services/images/thumbnail_cache.rs` — creates `app_data_dir/thumbnails/` dir on init  |

### Security & Privacy

- **DB path is always `app_data_dir/emmm2.db`** — no user-supplied override accepted. `app_data_dir` is resolved by Tauri's OS-native API (never from environment variables or user input).
- **Single-instance IPC socket** is OS-managed (named pipe on Windows) and does not expose any network port.
- **No data leaves the machine** during bootstrap; the background updater only performs a read-only HTTP GET to the configured GitHub Releases endpoint.
- **Log files** must not contain user file paths or mod folder names — only structured operational messages.
- **Migration failures** must not expose raw SQL error strings in the user-facing dialog — only a generic "Database initialization failed" message with a log reference.

---

## 4. Dependencies

- **Blocks**: All other epics — this is the root bootstrap that must succeed before any feature is usable.
- **Blocked by**: None — this is the root epic.
