# Epic 25: Scan Engine

## 1. Executive Summary

- **Problem Statement**: Users with existing mod libraries need the app to discover all mod folders automatically — but a naive recursive walk is slow, blocks the UI, and misses folders that need normalization (disabled prefix, variant structure) or signal extraction (INI hashes, folder name tokens) for Deep Matcher categorization.
- **Proposed Solution**: An async Tokio task that walks the `mods_path` with `walkdir` (bounded depth), extracts `FolderSignals` (name tokens, INI section tokens) per discovered mod, emits `scan_progress` events at ≤ 2s intervals, supports cancellation via a `CancellationToken`, and stores its running state in an `Arc<Mutex<ScanState>>`.
- **Success Criteria**:
  - A scan of 1,000 mod folders completes in ≤ 30s on SSD (measured on a benchmark library).
  - Progress events stream to the frontend within ≤ 2s of each batch of 50 folders being processed.
  - Permission-denied paths are skipped with a `warn` log — the scan always reaches `Completed` state (never hangs on a denied path).
  - Cancel command halts the walker within ≤ 1s of receipt.
  - Scan results include thumbnail paths for ≥ 95% of mod folders that actually contain a `preview.png` or `preview.jpg`.

---

## 2. User Experience & Functionality

### User Stories

#### US-25.1: Full Filesystem Scan

As a user, I want the app to scan my entire mods directory, so that it finds and categorizes all installed mods regardless of folder depth.

| ID        | Type        | Criteria                                                                                                                                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-25.1.1 | ✅ Positive | Given an active game with a valid `mods_path`, when "Scan Now" is triggered, then the scanner recursively walks the directory (max depth: 8) and identifies all valid 3DMigoto mod folders containing `.ini` files |
| AC-25.1.2 | ✅ Positive | Given a scan in progress on 500+ folders, then `scan_progress` events with `{scanned_count, total_estimate, current_path}` are emitted every ≤ 50 files processed, updating a progress bar in the UI               |
| AC-25.1.3 | ❌ Negative | Given a sub-directory the OS denies read access to, the scanner logs a `warn`-level entry with the path and moves on — it does not crash or hang; the scan completes `Completed` state regardless                  |
| AC-25.1.4 | ⚠️ Edge     | Given a `mods_path` containing symlinks that form cycles, then the walker's `follow_links = false` setting prevents infinite recursion — depth limit also serves as a backstop                                     |

---

#### US-25.2: Cancel Scan

As a user, I want to stop a long-running scan, so that I can regain control of the application.

| ID        | Type        | Criteria                                                                                                                                                                                   |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-25.2.1 | ✅ Positive | Given a scan is running, when `invoke('cancel_scan')` is called, then the `CancellationToken` is signaled; the walker halts within ≤ 1s; `ScanState` transitions to `Cancelled`            |
| AC-25.2.2 | ✅ Positive | Given cancellation, the partial scan results collected so far are NOT discarded — they can still be retrieved via `get_scan_result` and processed by the Deep Matcher over the partial set |
| AC-25.2.3 | ⚠️ Edge     | Given `cancel_scan` is called after the scan has already `Completed`, then the command returns an `AlreadyCompleted` status — no error thrown                                              |

---

#### US-25.3: Thumbnail Extraction

As a system, I want the scanner to note preview image paths for each mod, so that the UI has thumbnail data after scanning without additional round-trips.

| ID        | Type        | Criteria                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-25.3.1 | ✅ Positive | Given a mod folder containing `preview.png` or `preview.jpg`, when scanned, then the `ScanResult` entry for that folder includes `thumbnail_path = Some(absolute_path)` |
| AC-25.3.2 | ✅ Positive | Given a mod folder with no image file, then `thumbnail_path = None` — the grid card shows a fallback icon                                                               |

---

#### US-25.4: Folder Signal Extraction

As a system, I want to extract tokenized signals from folder names and INI files during scanning, so that the Deep Matcher has rich context to auto-categorize mods.

| ID        | Type        | Criteria                                                                                                                                                                              |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-25.4.1 | ✅ Positive | Given a scanned mod folder, the `FolderSignals` include: `name_tokens` (folder name split on `_`, space, camelCase) after stopword removal using the active GameSchema                |
| AC-25.4.2 | ✅ Positive | Given a mod folder containing `.ini` files, then `ini_section_tokens` (extracted from `[SectionName]` headers) and `ini_content_tokens` (key values) are included in `FolderSignals`  |
| AC-25.4.3 | ⚠️ Edge     | Given a mod folder with a malformed or binary `.ini` file, then `FolderSignals.ini_section_tokens` is empty for that file — no panic, the signal extraction continues for other files |

---

### Non-Goals

- Scan does not write DB records — it produces `ScanResult` data structures used by Epic 26 (Deep Matcher) and Epic 27 (Sync DB) to update the DB.
- Scan does not produce thumbnail images — only records existing image paths.
- No incremental scan in this epic — only full scan; incremental updates come from Epic 28 (File Watcher).

---

## 3. Technical Specifications

### Architecture Overview

```
ScanState: Running | Completed | Cancelled | Idle
  stored in: Arc<Mutex<ScanState>> (Tauri managed state)
  progress: (scanned: u32, estimate: u32, last_path: PathBuf)

start_scan(game_id) → Result<(), CommandError>:
  1. Check ScanState != Running (error if already running)
  2. Set ScanState = Running
  3. Spawn Tokio task:
     walker = WalkDir::new(mods_path).max_depth(8).follow_links(false).into_iter()
     for entry in walker:
       if token.is_cancelled(): break (→ ScanState = Cancelled)
       if error(PermissionDenied): warn!() + continue
       if classify(entry) == ModPackRoot:
         signals = extract_folder_signals(entry, game_schema)
         thumbnail = find_thumbnail(entry)
         push ScanResult { path, signals, thumbnail, is_enabled }
       if scanned_count % 50 == 0: emit('scan_progress', { scanned_count, estimate, last_path })
     Set ScanState = Completed + store Vec<ScanResult>

cancel_scan() → (): token.cancel()
get_scan_result() → Vec<ScanResult>
```

### Integration Points

| Component         | Detail                                                                                                           |
| ----------------- | ---------------------------------------------------------------------------------------------------------------- |
| Walker            | `walkdir` crate — `WalkDir::new(mods_path).max_depth(8).follow_links(false)`                                     |
| Cancellation      | `tokio_util::CancellationToken` — stored in `Arc` alongside `ScanState`                                          |
| Signal Extraction | `services/scanner/signals.rs::extract_folder_signals` — uses GameSchema stopwords (Epic 09)                      |
| Progress Events   | `window.emit('scan_progress', payload)` every 50 entries                                                         |
| Frontend          | `scannerStore.ts` listens to `scan_progress` → progress bar; `invoke('get_scan_result')` after `Completed` event |
| Deep Matcher      | Consumes `Vec<ScanResult>` (Epic 26)                                                                             |

### Security & Privacy

- **`mods_path` is validated** before scan starts — `canonicalize()` must succeed (no scan of invalid paths).
- **`follow_links = false`** — no symlink traversal; cycle attack is impossible.
- **Scan never mutates any file** — read-only operation; no rename, no write during walk.

---

## 4. Dependencies

- **Blocked by**: Epic 02 (Game Management — valid `mods_path`), Epic 09 (Object Schema — GameSchema for stopword tokenization).
- **Blocks**: Epic 26 (Deep Matcher — consumes `Vec<ScanResult>`), Epic 27 (Sync DB — writes scan results to DB).
