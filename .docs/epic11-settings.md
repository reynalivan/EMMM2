# Epic 11: Settings & System Infrastructure

**Focus:** Centralizing game configuration management, launch automation, privacy security, and global data synchronization using secure data persistence (Rust Serde) and _Atomic File Operations_.

## Dependencies

| Direction    | Epic   | Relationship                                                 |
| ------------ | ------ | ------------------------------------------------------------ |
| ⬆ Upstream   | Epic 1 | Reuses `validate_instance` service for game CRUD             |
| ⬆ Upstream   | Epic 7 | PIN management UI                                            |
| ⬆ Upstream   | Epic 8 | Orphan cleanup for `collection_items`                        |
| ⬇ Downstream | All    | Provides global config to all epics via `tauri-plugin-store` |

## Cross-Cutting Requirements

- **Game CRUD:** Add/Edit/Delete game reuses `validate_instance` from E1's service. Not a separate implementation.
- **Naming:** Uses `launcher_path` (renamed from `loader_path` per TRD).
- **Atomic Write:** Config writes use temp file + rename pattern. Never write directly to `config.json`.
- **Forms:** All settings forms use `React Hook Form` + `Zod` for validation.
- **Error Log Viewer:** Settings page includes a log viewer tab reading from `tauri-plugin-log` files.
- **Maintenance:** Weekly scheduled task cleans orphaned `collection_items`, stale thumbnails, and empty trash entries > 30 days.

---

## 1. User Stories & Acceptance Criteria

### US-11.1: Comprehensive Game Management (CUD)

**As a** user, **I want to** manage a list of modification directories for various games, **So that** I can quickly switch and activate mods on different games.

- **Acceptance Criteria:**
  - **Game Configuration Structure**: Users can add/edit game entries consisting of:
    - `id`: UUID (Auto-generated).
    - `name`: Display Name (e.g., "Genshin Impact").
    - `path`: Mod folder location (Validated).
    - `game_exe`: Game executable location.
    - `loader_exe`: 3DMigoto Loader location.
  - **Validation (Rust)**: The system rejects input if the path is invalid (`!Path::new(path).exists()`).
  - **Full Rescan**: A "Rescan Library" button that re-triggers _Indexing_ (Epic 2) for the selected game.

### US-11.2: Privacy & Security (Safe Mode Control)

**As a** user, **I want to** secure sensitive content, **So that** my privacy is maintained when using the application in public places or while _streaming_.

- **Acceptance Criteria:**
  - **PIN Hashing**: The PIN is stored in a **Hash (Argon2)** format, not plain text, within `config.json`.
  - **Keyword Management**: A list of keywords (e.g., "naked", "nsfw") is stored as a `Vec<String>`.
  - **Exclusive Toggle**: A `force_exclusive_mode` (bool) option to determine Safe Mode behavior.

### US-11.3: Maintenance Hub

**As a** user, **I want** tools to clean up junk data, **So that** the application remains lightweight and does not waste disk space.

- **Acceptance Criteria:**
  - **Trash Management**: An "Empty Trash" button permanently deletes the contents of the `./app_data/trash/*` folder.
  - **Cache Purge**: A "Clear Thumbnail Cache" button deletes image files in `cache/thumbnails` that are older than 30 days (LRU).
  - **Orphan Cleaner**: Removes database entries for files that no longer exist on the disk (Sync Check).

---

## 2. Technical Specifications (Settings Logic)

### A. Data Persistence (Rust Serde)

Application configurations are stored as Rust structs serialized to JSON.

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GameConfig {
    id: String,
    name: String,
    mod_path: PathBuf,
    game_exe: PathBuf,
    loader_exe: PathBuf,
    launch_args: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppSettings {
    theme: String, // "dark", "light", "system"
    language: String,
    games: Vec<GameConfig>,
    active_game_id: Option<String>,
    safe_mode: SafeModeConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SafeModeConfig {
    enabled: bool,
    pin_hash: Option<String>,
    keywords: Vec<String>,
}
```

### B. Atomic Configuration Save

Prevents `config.json` data corruption if the application crashes while writing.

```rust
use std::fs;
use std::io::Write;

fn save_config_atomically(settings: &AppSettings) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(settings)?;
    let path = PathBuf::from("config.json");
    let tmp_path = path.with_extension("tmp");

    // 1. Write to temp file
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?; // Ensure flush to disk

    // 2. Atomic Rename (Replace)
    fs::rename(&tmp_path, &path)?;

    Ok(())
}
```

### C. Maintenance Task (Disk Cleanup)

```rust
fn clean_old_cache(cache_dir: &Path, max_age_days: u64) {
    let cutoff = SystemTime::now() - Duration::from_secs(max_age_days * 86400);

    for entry in walkdir::WalkDir::new(cache_dir) {
        if let Ok(e) = entry {
            if let Ok(meta) = e.metadata() {
                if let Ok(accessed) = meta.accessed() {
                    if accessed < cutoff {
                        let _ = fs::remove_file(e.path());
                    }
                }
            }
        }
    }
}
```

---

## 3. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **CRUD Game**: Add Game → Save → Config updated → Sidebar refreshes.
- [ ] **PIN Security**: Set PIN → `config.json` shows Argon2 hashed value (not "1234").
- [ ] **Cache Purge**: Click "Clear Cache" → Files > 30 days deleted.
- [ ] **Error Log Viewer**: Settings > Logs tab → Shows recent log entries with level filter (INFO/WARN/ERROR).

### 2. Negative Cases (Error Handling)

- [ ] **Lockout**: **5** wrong PIN attempts → UI blocks input for 60s (standardized with E7).
- [ ] **Invalid Path**: User inputs "Z:/FakePath" → Input turns red via Zod validation → Save button disabled.
- [ ] **Duplicate Game Path**: Adding path that already exists → Warning "Already registered" (reuses E1 service).

### 3. Edge Cases (Stability)

- [ ] **Corrupt Config**: Delete `config.json` → App restarts → Fresh default config generated safely.
- [ ] **Atomic Write**: Kill app during Save → `config.json` remains valid (temp + rename pattern).
- [ ] **Maintenance Run**: Orphan cleanup runs weekly → Stale collection_items removed → No user data lost.

### 4. Technical Metrics

- [ ] **Save Latency**: Atomic write completes in **< 30ms**.
- [ ] **Hashing**: Argon2 PIN verification takes **< 100ms**.
- [ ] **Accessibility**: All settings forms have labels. Tab navigation works through all sections.
