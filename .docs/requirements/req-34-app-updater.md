# Epic 34: System Maintenance & Dynamic Updates

## 1. Executive Summary

- **Problem Statement**: EMMM2 must stay current on two separate tracks — app binary updates (bug fixes, new features) and dynamic schema/data assets (new game characters, resource pack databases) — without forcing users to re-download the full binary for data-only changes.
- **Proposed Solution**: Two update pipelines: (1) `tauri-plugin-updater` v2 for binary app updates via GitHub Releases, with Ed25519 signature verification, a streamed progress modal, and user-confirmed install; (2) a dynamic asset sync that lazily fetches `schema.json`, `master_db.json`, and Resource Pack files per game from the EMMM2-Assets CDN using `reqwest` with `If-Modified-Since`/`ETag` caching headers, exponential backoff on failure, and atomic cache overwrites. Debug builds (`v0.0.0`) are ignored by update logic to protect developer environments.
- **Success Criteria**:
  - Background update check completes within ≤ 5s of app start (non-blocking, `tokio` async task).
  - Metadata check adds ≤ 500ms to startup time.
  - Update download progress streams at ≥ 1 update/s for files > 5MB.
  - Dynamic schema fetch completes in ≤ 3s on a 10Mbps connection (3s `reqwest` timeout).
  - Rate limit hits trigger exponential backoff with ≤ 3 retries — no silent failure.
  - All update paths fail gracefully offline — cached schema used if present; bundled fallback used otherwise; 0 crashes.
  - Mid-download abort (user closes modal) leaves old binary intact — no corrupt intermediate state.

---

## 2. User Experience & Functionality

### User Stories

#### US-34.1: App Binary Update

As a user, I want the app to notify me when a new version is available and install it with one click, so that I always have the latest features without manually checking GitHub.

| ID        | Type        | Criteria                                                                                                                                                                                                                          |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-34.1.1 | ✅ Positive | Given the app starts and background check finds a new release, then an "Update Available (v{x.y.z})" toast appears at the top bar within ≤ 5s of startup — with a "View Changelog" button                                         |
| AC-34.1.2 | ✅ Positive | Given Settings > Maintenance, when I click "Check for Updates", then the current version, new version, and formatted release notes (Markdown-rendered) are displayed in ≤ 5s                                                      |
| AC-34.1.3 | ✅ Positive | Given I click "Download and Install", then a progress modal shows "Downloading... {X}/{Y} MB" with a streaming progress bar (≥ 1 update/s); on completion, a "Restart Now" button appears; clicking it calls `process.relaunch()` |
| AC-34.1.4 | ✅ Positive | Given the update file exceeds 50MB, then before download begins a confirmation dialog shows "This update is {size}MB. Download now?" — requiring explicit user approval                                                           |
| AC-34.1.5 | ❌ Negative | Given the updater endpoint is unreachable (no internet), then "Check for Updates" shows "Could not reach update server — check your connection"; the app continues running normally, no crash                                     |
| AC-34.1.6 | ❌ Negative | Given update signature verification fails (corrupted or tampered download), then the install is aborted, the partial file is deleted, and a toast shows "Update Failed: Invalid Signature — download aborted for security"        |
| AC-34.1.7 | ⚠️ Edge     | Given the user clicks "Close" during download (before install), then the partial download is discarded; the old binary remains intact — no corrupt intermediate state                                                             |
| AC-34.1.8 | ⚠️ Edge     | Given the running app version is `v0.0.0` (debug/dev build), then the update check is skipped entirely — no notification, no auto-install; this protects developer environments from unintended overwrites                        |

---

#### US-34.2: Dynamic Asset Syncing (Schema & Resource Pack)

As a system, I want to lazily fetch game schemas and entity databases from CDN, so that the app binary stays small and new characters/games are supported without a binary update.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                   |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------ |
| AC-34.2.1 | ✅ Positive | Given the app starts, it checks `manifest.json` on GitHub CDN using `If-Modified-Since`/`ETag` headers; only downloads if the remote version is newer — adds ≤ 500ms to startup                                                                            |
| AC-34.2.2 | ✅ Positive | Given a user switches to a game whose `schema.json` is missing locally, then the app fetches it from `https://raw.githubusercontent.com/reynalivan/EMMM2-Assets/main/{game_id}/schema.json` in ≤ 3s; cached to `app_data_dir/assets/{game_id}/schema.json` |
| AC-34.2.3 | ✅ Positive | Given the remote DB version is newer than local, then the new data is downloaded, parsed, and upserted into the SQLite `metadata` table — a "New Data Available" notification appears                                                                      |
| AC-34.2.4 | ✅ Positive | Given Settings > Maintenance has a "Sync Assets" button, when clicked, then all schemas and resource packs for configured games are re-fetched in parallel (`rayon` per game_id) — results show `{ game_id, status: Ok                                     | Cached | Failed }` per game |
| AC-34.2.5 | ❌ Negative | Given the GitHub API rate limit is hit (HTTP 429), then the request is retried with exponential backoff (1s → 2s → 4s, max 3 retries) — if all 3 retries fail, the cached version is used and a toast shows "Rate limited — using cached data"             |
| AC-34.2.6 | ❌ Negative | Given the app is offline when fetching a schema, then the last cached version is used if present — if no cache exists, a bundled fallback schema is used and a warning banner shows "Could not fetch latest data — using bundled fallback"                 |
| AC-34.2.7 | ⚠️ Edge     | Given the fetched `schema.json` fails JSON validation (malformed), then the previous cached version is kept intact — no overwrite of a valid cache with bad data; a toast shows "Asset sync failed: invalid schema format"                                 |
| AC-34.2.8 | ⚠️ Edge     | Given the app is killed mid-download of a schema, then on next launch, the incomplete `.tmp` file is detected and deleted before fetching again — no stale partial JSON used as cache                                                                      |

---

#### US-34.3: Lazy Asset Fetching (Icons & Thumbnails)

As a user, I want missing game icons or UI assets to be fetched automatically when needed, so that the installer stays small while the UI always looks complete.

| ID        | Type        | Criteria                                                                                                                                                                                                                             |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-34.3.1 | ✅ Positive | Given loading an icon file (e.g., `Element_Dendro.png`) results in File Not Found, then a background download is triggered from the asset CDN; on success, the icon is cached to `app_data_dir/cache/assets/` and reloaded in the UI |
| AC-34.3.2 | ✅ Positive | Given the icon is successfully cached, then subsequent loads come from the local cache — no repeated CDN requests                                                                                                                    |
| AC-34.3.3 | ❌ Negative | Given the icon download fails (offline or CDN error), then the UI shows a default fallback icon — no broken image element, no crash                                                                                                  |

---

### Non-Goals

- No downgrade / rollback of the binary — forward updates only.
- No delta patching — full file replacement only.
- No auto-install without user confirmation — always "Download and Install" button-gated.
- Dynamic asset sync does not update game engine files (3DMigoto DLLs) — only EMMM2 data files.
- No per-game binary updater — the single app binary serves all games.

---

## 3. Technical Specifications

### Architecture Overview

```
Binary Update (tauri-plugin-updater v2):
  useAppUpdater state: Idle → Checking → Available → Downloading → ReadyToRestart → Done

  check_update():
    if version == "0.0.0": return None  // debug guard
    updater.check() → Option<Update { version, body, notes, download_size }>
    if download_size > 50MB: show confirm dialog first

  install_update(update):
    update.downloadAndInstall(|downloaded, total| {
      emit('update:progress', { downloaded, total })  // ≥1/s
    })
    → on complete: emit('update:ready')
  Frontend: "Restart Now" → process.relaunch()

Dynamic Asset Sync:
  on_startup():
    check_manifest() — If-Modified-Since / ETag → only fetch if newer → adds ≤500ms
    for_each_game: fetch_game_schema(game_id) lazily

  fetch_game_schema(game_id) → Result<GameSchema>:
    url = "https://raw.githubusercontent.com/reynalivan/EMMM2-Assets/main/{game_id}/schema.json"
    result = retry_with_backoff(max_retries=3, initial_delay=1s, factor=2x):
      reqwest::get(url).timeout(3s)
    if HTTP 429: exponential backoff
    → validate JSON (serde)
    → if valid: write atomically to schema.tmp → rename to schema.json
    → if invalid: return Err, leave old cache intact
    → if offline: load from cache / bundled fallback

  sync_all_assets(game_ids) → Vec<AssetSyncResult>:
    rayon::par_iter(game_ids) → fetch_game_schema per game
    → { game_id, status: Ok(version) | Cached | Failed(reason) }

Lazy Icon Fetch:
  on NotFound(icon_path):
    background_spawn: fetch CDN url → save to app_data_dir/cache/assets/
    → invalidate React Query ['assets', icon_key]

Startup temp file cleanup:
  on_startup: glob(app_data_dir/assets/**/*.tmp) → delete all
```

### Integration Points

| Component          | Detail                                                                                             |
| ------------------ | -------------------------------------------------------------------------------------------------- |
| Binary Updater     | `tauri-plugin-updater` v2 — endpoint + `pubkey` (Ed25519) in `tauri.conf.json` → GitHub Releases   |
| Asset CDN          | `reqwest::Client` with 3s timeout; `If-Modified-Since` / `ETag` headers; retry backoff             |
| Schema Cache       | `{app_data_dir}/assets/{game_id}/schema.json` — validated before overwrite; atomic `.tmp` → rename |
| Bundled Fallback   | Shipped in app bundle at `resources/schemas/{game_id}_fallback.json`                               |
| DB Upsert          | New metadata rows upserted via `INSERT OR REPLACE INTO metadata` — incremental, no full rebuild    |
| Frontend State     | `useAppUpdater.ts` — Zustand state machine for binary update flow                                  |
| Download Size Gate | > 50MB → confirmation dialog before any download starts                                            |

### Security & Privacy

- **Binary updates are Ed25519-signed** — `tauri-plugin-updater` enforces signature verification; signature failure aborts and deletes partial download.
- **Schema source restricted to `github.com/reynalivan`** — no arbitrary URL schema fetch; URL is hardcoded in Rust, not frontend-configurable.
- **Fetched JSON validated** (serde) before cache overwrite — malformed schema cannot corrupt local cache.
- **`v0.0.0` guard** prevents debug binaries from being overwritten by production update channel.
- **Atomic cache writes** (`.tmp` → rename) prevent partial JSON being used if interrupted.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — `app_data_dir` resolved), Epic 02 (Game Management — `game_id` context for schema fetch).
- **Blocks**: Epic 09 (Schema load path — dynamic schemas overwrite bundled schemas when available), Epic 43 (Dynamic KeyViewer — Resource Pack updates trigger KeyViewer regeneration).
