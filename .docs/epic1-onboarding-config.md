# Epic 1: Onboarding & System Configuration (Strict Final)

**Focus:** Handles **EMM2** application initialization, setup mode selection (XXMI vs Manual), game folder integrity validation, and creating the initial `app_settings` in the SQLite database using the **Rust/Tauri** architecture.

## Dependencies

| Direction    | Epic    | Relationship                            |
| ------------ | ------- | --------------------------------------- |
| ⬇ Downstream | Epic 2  | Provides `games.path` for mod scanning  |
| ⬇ Downstream | Epic 3  | Provides game context for sidebar       |
| ⬆ Upstream   | Epic 11 | Reuses game validation service for CRUD |

## Cross-Cutting Requirements

- **Logging:** All validation steps log result to `tauri-plugin-log` at INFO/WARN level.
- **State:** On successful config load, initialize `appStore` (Zustand) with `active_game_id` and `safe_mode_enabled`.
- **TanStack Query:** After game addition, call `queryClient.invalidateQueries(['games'])`.
- **Forms:** Manual setup form uses `React Hook Form` + `Zod` for path validation.

---

## A. User Stories & Logic Detail

### US-1.1: Setup Mode Selection

**As a** new user, **I want to** choose the setup method on the application's first screen, **So that** I can immediately use the application without complicated manual configuration if I have followed community standards.

- **Logic:**
  1.  **Startup Check (Rust):** When the application is opened, the Rust backend (`ConfigService`) loads settings from the SQLite database. (It also runs a one-time migration from `config.json` if the DB is empty).
  2.  **Condition:**
      - If DB `games` array is not empty → **Skip** this screen, send `NAVIGATE_DASHBOARD` event.
      - If DB `games` array is empty → Show **Welcome Screen**.
  3.  **Multi-Instance Guard (Tauri Plugin):** If another EMMM2 instance is detected → focus existing window → exit new instance silently.
  4.  **UI Elements:**
      - Primary Button: **"XXMI Auto-Detect"** (Runs heuristic scanning).
      - Secondary Button: **"Add Game Manually"** (Opens OS standard file picker).

### US-1.2: XXMI Auto-Discovery Algorithm

**As the** system, **I must** scan the standard XXMI folder structure and only accept folders that meet 3DMigoto technical requirements, **So that** the user doesn't need to enter paths one by one.

- **Input:** Root Folder Path (from Tauri Native Folder Picker).
- **Target Subfolders:** Static list `["GIMI", "SRMI", "WWMI", "ZZMI", "EFMI"]`.
- **Iteration Logic (Rust Thread):**
  - Iterate through each `target` in the static list.
  - Construct `full_path = root.join(target)`.
  - Run the `validate_instance(full_path)` function (See Section B).

- **Result Handling:**
  - If `validate_instance` returns **Ok(GameInfo)**: Add to `valid_games` vector.
  - If **Err**: Ignore that subfolder (Silent Fail).

- **Completion:**
  - If `valid_games` is empty: Show Error Modal _"No valid 3DMigoto instances found in standard XXMI folders."_
  - If `valid_games` > 0:
    1.  Save settings via `ConfigService::save_settings` (SQLite).
    2.  Redirect to Dashboard.

### US-1.3: Manual Path Setup Logic

**As a** manual user, **I want to** point to a specific folder and have the system validate it with the same rules, **So that** installation flexibility is maintained (e.g., installing on Drive D:).

- **Input:**
  1.  `GameType` Dropdown (Enum: GIMI, SRMI, WWMI, ZZMI, EFMI).
  2.  Folder Path (from `dialog::open`).

- **Logic:**
  - Run the `validate_instance(selected_path)` function.

- **Result Handling:**
  - If **Ok**: Add to `ConfigService` (SQLite) -> Redirect Dashboard.
  - If **Err(Reason)**: Show Inline Error in UI: _"Invalid Folder: [Reason]. Missing core files (d3dx.ini, d3d11.dll) or /Mods folder."_

---

## B. Technical Specifications (Rust/Tauri Implementation)

### 1. Validation Algorithm (Rust Native)

Validation logic is run on the Rust Backend for maximum I/O speed and type safety.

```rust
use crate::database::models::GameInfo;
use std::path::Path;

/// Return structure if validation is successful (from database::models)
pub struct GameInfo {
    pub path: String,
    pub launcher_path: String,
    pub mods_path: String,
}

pub fn validate_instance(path: &Path) -> Result<GameInfo, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    // RULE 1: /Mods folder is mandatory
    let mods_path = path.join("Mods");
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err("Missing /Mods folder".to_string());
    }

    // RULE 2: Core 3DMigoto files
    if !path.join("d3dx.ini").exists() {
        return Err("Missing core file: d3dx.ini".to_string());
    }
    if !path.join("d3d11.dll").exists() {
        return Err("Missing core file: d3d11.dll".to_string());
    }

    // RULE 3: Find launcher .exe (prefer names containing "loader")
    let exe_files: Vec<_> = std::fs::read_dir(path)
        .map_err(|e| format!("Cannot read directory: {e}"))?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
        })
        .map(|e| e.path())
        .collect();

    if exe_files.is_empty() {
        return Err("No .exe launcher found".to_string());
    }

    let launcher = exe_files
        .iter()
        .find(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains("loader")
        })
        .unwrap_or(&exe_files[0])
        .clone();

    Ok(GameInfo {
        path: path.to_string_lossy().to_string(),
        launcher_path: launcher.to_string_lossy().to_string(),
        mods_path: mods_path.to_string_lossy().to_string(),
    })
}
```

### 2. Configuration Data Structure (SQLite via ConfigService)

Configurations are stored using SQLite (`sqlx` plugin). The structures are modularized in `models.rs`. The `ConfigService` ensures all state changes are written safely to the database and provides in-memory synchronized access.

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub theme: String, // "dark", "light", "system"
    pub language: String,
    pub games: Vec<GameConfig>,
    pub active_game_id: Option<String>,
    pub safe_mode: SafeModeConfig,
    pub ai: AiConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: String, // "Genshin", "StarRail", "ZZZ", "Wuthering"
    pub mod_path: PathBuf,
    pub game_exe: PathBuf,
    pub loader_exe: Option<PathBuf>,
    pub launch_args: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafeModeConfig {
    pub enabled: bool,
    pub pin_hash: Option<String>,
    pub keywords: Vec<String>,
    pub force_exclusive_mode: bool,
}
```

### US-1.4: Multi-Instance Prevention

**As a** user, **I want** only one EMMM2 instance running at a time, **So that** I avoid file conflicts and database locking.

- **Implementation:** Use `tauri-plugin-single-instance`.
- **Behavior:**
  - Second instance detects first → sends focus signal → exits immediately.
  - First instance receives signal → `window.set_focus()` → brings to foreground.

### US-1.5: Duplicate Game Guard

**As a** user, **I want** the system to prevent me from adding the same game path twice, **So that** I don't create data conflicts.

- **Logic:**
  1.  Before inserting into `games` table, query: `SELECT id FROM games WHERE path = ?` or check existing `ConfigService` in-memory vector.
  2.  If result exists → return `Err("This game path is already registered as '{name}'.")` → Show warning Modal.
  3.  If not exists → proceed with update.

---

## C. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Fresh Install Algorithm**: App Start (Empty DB) -> ConfigService returns Default -> UI shows Welcome Screen -> User clicks "Auto Detect" -> Rust scans successfully -> DB updated -> Redirect Dashboard.
- [ ] **Manual Setup Parsing**: User selects `D:\Games\Genshin Impact` -> `validate_instance` returns Ok -> Game added to SQLite DB via `ConfigService`.
- [ ] **Persistence Check**: Kill App -> Restart -> ConfigService loads from DB -> Takes user strictly to Dashboard (No Welcome Screen).
- [ ] **Legacy Migration**: On first boot, if DB is empty but `config.json` exists, settings are migrated to SQLite effortlessly.

### 2. Negative Cases (Error Handling)

- [ ] **Root Validation**: User selects empty folder for Auto-Detect → Rust returns empty Vector → UI shows "No Instances Found" modal.
- [ ] **Bad Folder Structure**: Manual add folder with `d3dx.ini` but no `/Mods` → Returns explicit error "Missing /Mods folder".
- [ ] **Missing DLL**: Folder has `/Mods` but missing `d3d11.dll` → Returns error "Critical file missing".
- [ ] **Duplicate Game Path**: User adds path that already exists in DB → Shows warning "Already registered" → No duplicate entry created.
- [ ] **Multi-Instance Rejected**: Launching second instance → first window gets focus → second exits.

### 3. Edge Cases (Stability & Robustness)

- [ ] **Mixed Slash Handling**: Windows Backslash (`\`) and Forward Slash (`/`) are normalized by Rust `PathBuf` automatically.
- [ ] **Permission Hell**: App run with low privileges tries to write Config to `Program Files` → Rust catches `PermissionDenied` → UI prompts "Run as Admin required".
- [ ] **SQLite Initialization**: `ConfigService::ensure_tables` guarantees tables exist before read/write operations on boot.
- [ ] **Loading State**: While auto-detect runs, show Shimmer overlay with status text "Scanning...".

### 4. Technical Metrics

- [ ] **Scan Latency**: Auto-detect scan on SSD finishes in **< 50ms**.
- [ ] **Startup Time**: Time from App Icon Double Click to Dashboard (Cold Start) **< 800ms** (if config exists in DB).
- [ ] **DB Reliability**: SQLite WAL mode prevents database locks during simultaneous read/writes.
