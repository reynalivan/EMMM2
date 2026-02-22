# Epic 10: Automation & Workflow Optimization (QoL)

**Focus:** Maximizing user convenience through integrated game launcher based on `std::process::Command`, content curation system (Favorite/Pin), and an intelligent mod randomizer feature with visual previews.

## Dependencies

| Direction  | Epic   | Relationship                                                      |
| ---------- | ------ | ----------------------------------------------------------------- |
| ⬆ Upstream | Epic 1 | Reads `games.launcher_path` for launch target                     |
| ⬆ Upstream | Epic 5 | Uses toggle service for randomizer apply                          |
| ⬆ Upstream | Epic 7 | Randomizer respects Safe Mode filter                              |
| References | Epic 2 | Shader conflict detection (owned by E2, referenced in randomizer) |

## Cross-Cutting Requirements

- **Launcher path:** Uses `launcher_path` field (renamed from `loader_path` per TRD).
- **Launch Args:** Uses `games.launch_args` (from E1 config). e.g., `-popupwindow`.
- **Randomizer Safety:** When `appStore.safeMode = true`, randomizer pool excludes `is_safe = false` mods.
- **Pin/Favorite:** Stored in `mods` table (`is_pinned`, `is_favorite`). UI sorts pinned items first.

---

## 1. User Stories & Acceptance Criteria

### US-10.1: Integrated Launch System (One-Click Play)

**As a** gamer, **I want** a single "Play" button that automatically manages the _loader_ and game, **So that** the process of starting the game is instant and hassle-free.

- **Acceptance Criteria:**
  - **Process Check:** The system detects `3DMigoto Loader.exe` using `sysinfo`. If not running, it runs as Administrator.
  - **Argument Support:** Supports custom launch arguments (such as `-popupwindow`) stored in `config.json`.
  - **Auto-Close:** Option to automatically close the Mod Manager (`app.exit(0)`) after the game has successfully launched.

### US-10.2: Smart Randomizer (Gacha Mode with Preview)

**As a** user, **I want** the system to select mods randomly with a preview first, **So that** I have full control before physical changes are applied.

- **Acceptance Criteria:**
  - **State Awareness:** Randomization only occurs on mods that match the current **Safe Mode** status.
  - **Ignore Rule:** Folders with a **dot prefix (`.`)** (Hidden/System mods) will never be selected.
  - **Idea Preview:** The system does not perform a _rename_ immediately. A confirmation _popup_ appears containing the **Thumbnail** and **Mod Name** of the randomized result.
  - **Action:**
    - **Apply:** Executes the `enable_only_this` logic (Epic 5).
    - **Re-roll:** Searches for another random mod candidate.

### US-10.3: Collection Management (Favorite & Pin)

**As a** user, **I want to** mark the best mods or pin important folders, **So that** they are always in an easily accessible position.

- **Acceptance Criteria:**
  - **Favorite:** Mark a mod as a favorite; this status is stored in `info.json` (`is_favorite: true`).
  - **Pinned:** Pinned folders will be forced to appear at the very top of the grid (`sort_order: -1`).
  - **Sync:** Favorite/Pin status is synchronized between the UI and JSON.

### US-10.4: Hash Conflict Detection

- **Feature**: The system scans `.ini` files within active mods. If two active mods target the same _Texture Hash_, the application will provide a "Conflict Detected" warning.
- **Benefit**: Prevents character glitches caused by two mods overwriting the same hash.

---

## 2. Technical Specifications (Automation Logic)

### A. Integrated Launcher Logic (Rust)

Uses `std::process::Command` for secure and controlled execution.

```rust
use std::process::Command;
use sysinfo::{System, SystemExt, ProcessExt};

fn launch_game(loader_path: &str, game_exe: &str, args: Vec<String>) -> Result<(), String> {
    let s = System::new_all();

    // 1. Check if Loader is running
    let loader_name = Path::new(loader_path).file_name().unwrap().to_str().unwrap();
    let is_running = s.processes_by_name(loader_name).next().is_some();

    if !is_running {
        // Run as Admin (Windows specific verb)
        #[cfg(target_os = "windows")]
        {
            Command::new("powershell")
                .args(&["start-process", loader_path, "-Verb", "RunAs"])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    // 2. Launch Game
    Command::new(game_exe)
        .args(args)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

### B. Intelligent Randomizer Pipeline (Rust Rand)

```rust
use rand::seq::SliceRandom;

fn pick_random_mod(mods: &[Mod], is_safe_mode: bool) -> Option<Mod> {
    let candidates: Vec<&Mod> = mods.iter()
        .filter(|m| !m.path.starts_with(".")) // Ignore hidden
        .filter(|m| if is_safe_mode { m.is_safe } else { true }) // Safety check
        .collect();

    let mut rng = rand::thread_rng();
    candidates.choose(&mut rng).cloned().cloned()
}
```

---

## 3. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Launch Flow**: Click "Play" → `3DMigoto Loader` starts (Admin) → Game EXE starts → App Auto-Closes.
- [ ] **Randomizer**: Click "Gacha Mod" → Dialog shows Preview → Click "Apply" → Mod Enabled, others Disabled.
- [ ] **Pinning Interaction**: Toggle "Pin" on folder → Folder moves to top of grid instantly.

### 2. Negative Cases (Error Handling)

- [ ] **Admin Denied**: User clicks Play → Declines UAC Prompt → Logs "Launch Cancelled" → Toast "Please allow Admin access".
- [ ] **Game Missing**: Configured `launcher_path` is invalid → Click Play → Toast: "Launcher Not Found" → Redirect to Settings (E11).
- [ ] **Operation Lock**: Randomizer blocked while another toggle operation runs.

### 3. Edge Cases (Stability)

- [ ] **Safe Randomizer**: SFW Mode → Randomizer only suggests SFW mods (NSFW filtered out).
- [ ] **Custom Arguments**: Launch with `-popupwindow` → Game opens in windowed mode correctly.
- [ ] **Mandatory Mods**: Randomizer avoids touching mods with `.` prefix (System/Fixed mods).
- [ ] **Empty Pool**: Randomizer with 0 eligible mods (all filtered) → Toast: "No mods available for randomization".

### 4. Technical Metrics

- [ ] **Launch Latency**: Process execution command issued in **< 100ms**.
- [ ] **Randomizer Logic**: Selection algorithm runs in **< 10ms** even with 10k items.
- [ ] **Accessibility**: All buttons (Play, Pin, Randomize) have ARIA labels and keyboard shortcuts.
