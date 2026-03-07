# KeyViewer & In-game Hotkeys Implementation Plan

> **Goal:** Provide an in-game KeyViewer overlay for 3DMigoto games showing keybinds for the detected character, plus OS-level hotkeys for Safe Mode toggle, Preset switching, and Folder switching.

**Architecture:** A new `services/keyviewer/` Rust module owns the offline pipeline (hash harvesting → matching → sentinel selection → file generation). A separate `services/hotkeys/` module handles OS-level global hotkey registration and action dispatching. Frontend settings UI in `src/features/settings/`. The generated `KeyViewer.ini` runs inside 3DMigoto at runtime with zero per-frame overhead.

**Tech Stack:** Rust (backend), Tauri IPC, SQLite (SQLx), `global-hotkey` crate, `windows-sys` (SendInput), existing INI parser, existing collections/privacy services.

---

## User Review Required

> [!IMPORTANT]
> **This is a massive epic (~6 phases).** Each phase is independently testable and shippable. I recommend implementing **Phase 1 → 2 → 3** first (pure backend, no UI), then Phase 4 (hotkeys), then Phase 5-6 (UI + E2E).

> [!WARNING]
> **New crate dependencies required:** `global-hotkey` for OS hotkeys, `windows-sys` for sending keystrokes. These need review.

> [!CAUTION]
> **Resource Pack JSON files** (`gimi.json`, `srmi.json`, etc.) need to be curated per game. They define character/object identity and known hashes. These are data files, not code — they must be created by someone with game modding knowledge. The plan assumes the schema; actual data population is out of scope.

---

## Phase 1: Foundation (Hash Harvester + DB)

### Component: Resource Pack — Already Exists!

The existing MasterDb JSON files at `src-tauri/resources/databases/` already serve as resource packs:

- `gimi.json` (72KB, ~100 characters, populated `hash_db`)
- `srmi.json` (42KB, populated `hash_db`)
- `wwmi.json` (21KB, populated `hash_db`)
- `zzmi.json` (26KB, partially populated `hash_db`)
- `efmi.json` (3KB, empty `hash_db`)

Each entry has `hash_db: { "Default": ["code_hash"], "SkinName": ["code_hash"] }` — this IS the `known_hashes`/`code_hash` data from req-42. **No separate resource pack files needed.**

The existing `MasterDb` loader already parses these. For KeyViewer, we add a thin accessor.

---

#### [NEW] [resource_pack.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/resource_pack.rs)

Thin adapter extracting hash data from existing `MasterDb` entries.

```rust
// Key types:
pub struct KvObjectEntry {
    pub name: String,
    pub object_type: String,
    pub code_hashes: Vec<String>,    // All hashes from hash_db (flattened)
    pub skin_hashes: HashMap<String, Vec<String>>,
    pub tags: Vec<String>,
    pub thumbnail_path: Option<String>,
}

// Functions:
pub fn extract_kv_entries(db: &MasterDb) -> Vec<KvObjectEntry>
```

---

#### [NEW] [harvester.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/harvester.rs)

Extracts `hash = XXXXXXXX` values from enabled mods' INI files. Extends the existing INI parser patterns.

```rust
// Key types:
pub struct HarvestedHash {
    pub hash: String,                   // The hex hash value (e.g. "a1b2c3d4")
    pub section_name: String,           // e.g. "TextureOverrideBody"
    pub file_path: PathBuf,
    pub mod_id: String,
    pub occurrence_count: u32,
}

pub struct HashHarvestResult {
    pub hashes: Vec<HarvestedHash>,
    pub file_signature: FileSignature,  // {size, mtime, fast_hash}
}

pub struct FileSignature {
    pub size: u64,
    pub mtime: u64,
    pub fast_hash: String,  // blake3 of first 4KB
}

// Section filter regex: TextureOverride*, ShaderOverride*, TextureOverrideIB*, etc.
// Denylist: configurable list of known global/shared sections
// Hash extraction regex: `hash\s*=\s*([0-9a-fA-F]+)` within matching sections

// Functions:
pub fn harvest_hashes_from_mod(mod_path: &Path, mod_id: &str) -> Result<HashHarvestResult, String>
pub fn harvest_hashes_from_ini(file_path: &Path, mod_id: &str) -> Result<Vec<HarvestedHash>, String>
pub fn compute_file_signature(file_path: &Path) -> Result<FileSignature, String>
pub fn should_rescan(old_sig: &FileSignature, new_sig: &FileSignature) -> bool
```

**Reuse:** Uses `services::ini::document::list_ini_files` to discover `.ini` files. Does NOT modify the existing `read_ini_document` function — hash extraction is a separate concern.

---

#### [NEW] [mod.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/mod.rs)

Module root for the keyviewer service.

```rust
pub mod resource_pack;
pub mod harvester;
pub mod matcher;      // Phase 2
pub mod generator;    // Phase 3
```

---

#### [MODIFY] [mod.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/mod.rs)

Add `pub mod keyviewer;` to the service module registry.

---

#### [NEW] [migration SQL](file:///e:/Dev/EMMM2NEW/src-tauri/migrations/20260301100000_keyviewer_tables.sql)

```sql
-- Hash index from active mods (rebuilt on each regeneration)
CREATE TABLE IF NOT EXISTS mod_hash_index (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL COLLATE NOCASE,
    mod_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    section_name TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    game_id TEXT NOT NULL,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_hash ON mod_hash_index(hash);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_game ON mod_hash_index(game_id);

-- Sentinel cache per object (persisted across runs)
CREATE TABLE IF NOT EXISTS object_sentinel_cache (
    code_hash TEXT NOT NULL,
    game_id TEXT NOT NULL,
    sentinel_hashes TEXT NOT NULL,  -- JSON array
    confidence REAL NOT NULL DEFAULT 0.0,
    last_updated TEXT NOT NULL,
    sources TEXT NOT NULL,          -- JSON array of {mod_id, file}
    PRIMARY KEY (code_hash, game_id)
);

-- Keybind cache (extracted from enabled mods per regeneration)
CREATE TABLE IF NOT EXISTS keybind_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code_hash TEXT NOT NULL,
    mod_id TEXT NOT NULL,
    action_label TEXT NOT NULL,
    bound_key TEXT NOT NULL,
    category TEXT,       -- "Variants" | "FX" | "UI" | "Debug"
    is_safe INTEGER NOT NULL DEFAULT 1,
    game_id TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_keybind_cache_code ON keybind_cache(code_hash);

-- File signature cache for incremental scanning
CREATE TABLE IF NOT EXISTS file_signature_cache (
    file_path TEXT PRIMARY KEY,
    file_size INTEGER NOT NULL,
    file_mtime INTEGER NOT NULL,
    fast_hash TEXT NOT NULL,
    game_id TEXT NOT NULL
);
```

---

### Tests for Phase 1

#### [NEW] [harvester_tests.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/tests/harvester_tests.rs)

```
Test cases:
1. harvest_basic_hash_extraction — INI with TextureOverride section containing hash, verify extracted correctly
2. harvest_shader_override_section — ShaderOverride section hashes extracted
3. harvest_ignores_non_override_sections — [Constants] hash ignored
4. harvest_denylist_filtering — Denied section patterns excluded
5. harvest_multiple_files — Multiple INI files in mod folder
6. harvest_file_signature — Correct size/mtime/blake3 computation
7. harvest_incremental_skip — No rescan when signature unchanged
8. harvest_encoding_handling — Shift-JIS INI files handled
```

Run: `cargo test --lib keyviewer::harvester`

#### [NEW] [resource_pack_tests.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/tests/resource_pack_tests.rs)

```
Test cases:
1. load_valid_pack — Valid JSON loads correctly
2. load_invalid_json — Malformed JSON returns error
3. code_hash_parsing — Hex string "0xA1B2C3D4" parsed to u32
4. empty_known_hashes — Entry with empty hash list loads fine
5. filter_by_game_id — Only entries for requested game returned
```

Run: `cargo test --lib keyviewer::resource_pack`

---

## Phase 2: Hash Matching + Sentinel Selection

#### [NEW] [matcher.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/matcher.rs)

Matches active-mod hashes against resource pack objects and selects optimal sentinels.

```rust
// Key types:
pub struct MatchResult {
    pub code_hash: u32,
    pub object_name: String,
    pub score: f32,
    pub sentinel_hashes: Vec<String>,
    pub confidence: f32,
}

pub struct MatchConfig {
    pub min_score_threshold: f32,       // Default: requires 1-2 strong intersections
    pub max_sentinels_per_object: usize, // Default: 3
    pub collision_object_threshold: u32, // Default: 3 (hash in ≥3 objects = high-collision)
    pub min_margin_percent: f32,         // Default: 0.15 (15%)
}

// Core algorithm:
// 1. For each resource pack object, compute I = active_hashes ∩ known_hashes
// 2. Score = Σ (base=10 per hash + log(1+occurrence) + rarity_bonus + hint_bonus)
// 3. Pick best object if score >= threshold
// 4. Ties: higher priority > higher score > stable order
// 5. Sentinel selection: top K hashes from I by weight, avoiding high-collision
// 6. High-collision: hash in ≥3 objects OR ambiguous across candidates

// Functions:
pub fn match_objects(
    active_hashes: &[HarvestedHash],
    resource_pack: &ResourcePack,
    config: &MatchConfig,
) -> Vec<MatchResult>

pub fn select_sentinels(
    intersection: &[&str],
    active_hashes: &[HarvestedHash],
    resource_pack: &ResourcePack,
    config: &MatchConfig,
) -> Vec<String>

pub fn is_high_collision(
    hash: &str,
    resource_pack: &ResourcePack,
    config: &MatchConfig,
) -> bool
```

### Tests for Phase 2

#### [NEW] [matcher_tests.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/tests/matcher_tests.rs)

```
Test cases:
1. match_single_object — One object, hashes intersect, correct score
2. match_priority_tiebreak — Equal score, higher priority wins
3. match_score_tiebreak — Equal priority, higher score wins
4. match_below_threshold — Score below threshold returns no match
5. sentinel_selection_top_k — Top 3 by weight selected
6. sentinel_avoids_collision — High-collision hash excluded
7. collision_detection_3_objects — Hash in 3+ objects flagged
8. collision_detection_margin — Hash ambiguous within margin flagged
9. empty_intersection — No matching hashes → no sentinels
10. rarity_bonus — Rare hash gets higher score
```

Run: `cargo test --lib keyviewer::matcher`

---

## Phase 3: File Generation Pipeline

#### [NEW] [generator.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/generator.rs)

Generates keybind text files and `KeyViewer.ini`.

```rust
// Functions:
pub fn generate_keybind_text(
    code_hash: u32,
    keybinds: &[KeybindEntry],
    safe_mode: bool,
) -> String

pub fn generate_fallback_text() -> String

pub fn generate_keyviewer_ini(
    matches: &[MatchResult],
    toggle_key: &str,
    ttl_seconds: f32,
    min_hold_seconds: f32,
) -> String

pub fn write_atomic(path: &Path, content: &str) -> Result<(), String>

pub fn discover_reload_key(game_path: &Path) -> String  // Reads d3dx.ini, defaults to "F10"

// Keybind text format:
// ─── <ObjectName> ───
// [Variants]
//   J - Toggle Variant 1
//   K - Toggle Variant 2
// [FX]
//   L - Toggle Effect
// ─── Preset: <name> ───

// KeyViewer.ini contains:
// - Global vars ($kv_on, $kv_active_code, $kv_active_priority, etc.)
// - Toggle key section
// - Per-object sentinel TextureOverride/ShaderOverride sections
// - Present section with TTL check + overlay draw
// - Anti flip-flop hysteresis logic
```

**Atomic write:** All generated files use `.tmp` → `fs::rename` pattern (same as used in `services/fs_utils/`).

### Tests for Phase 3

#### [NEW] [generator_tests.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/keyviewer/tests/generator_tests.rs)

```
Test cases:
1. generate_keybind_text_basic — Correct format with sections
2. generate_keybind_safe_mode — NSFW keybinds excluded when safe=true
3. generate_fallback — Valid fallback text
4. generate_ini_single_object — INI with one sentinel object
5. generate_ini_multiple_objects — INI with multiple sentinel objects
6. generate_ini_toggle_key — Custom toggle key
7. atomic_write_success — File written atomically
8. atomic_write_no_partial — Interrupted write leaves old file intact (simulated)
9. discover_reload_key_found — Reads F10 from d3dx.ini
10. discover_reload_key_missing — Returns default F10 when d3dx.ini missing
11. keybind_text_max_lines — Truncates at 60 lines with "..."
12. keybind_text_max_bytes — Truncates at 8KB
```

Run: `cargo test --lib keyviewer::generator`

---

## Phase 4: OS Hotkey Listener + In-game Actions

#### [NEW] Cargo.toml additions

```toml
# In-game hotkeys (req-42)
global-hotkey = "0.6"
```

For Windows keystroke sending (SendInput), use `windows-sys` which Tauri already depends on transitively.

---

#### [NEW] [hotkeys/mod.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/hotkeys/mod.rs)

OS-level hotkey manager.

```rust
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    bindings: HashMap<HotKeyId, HotkeyAction>,
    cooldown_ms: u64,
    last_triggered: HashMap<HotkeyAction, Instant>,
}

pub enum HotkeyAction {
    ToggleSafeMode,
    NextPreset,
    PrevPreset,
    NextFolder,
    PrevFolder,
    ToggleStatusOverlay,
}

pub struct HotkeyConfig {
    pub safe_toggle: String,    // Default: "F5"
    pub next_preset: String,    // Default: "F6"
    pub prev_preset: String,    // Default: "Shift+F6"
    pub next_folder: String,    // Default: "F8"
    pub prev_folder: String,    // Default: "Shift+F8"
    pub toggle_status: String,  // Default: "F7"
    pub cooldown_ms: u64,       // Default: 500
}

// Functions:
pub fn register_hotkeys(config: &HotkeyConfig) -> Result<HotkeyManager, String>
pub fn unregister_all(manager: &HotkeyManager) -> Result<(), String>
```

---

#### [NEW] [hotkeys/actions.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/hotkeys/actions.rs)

Action handlers that orchestrate workspace changes + regeneration + reload.

```rust
// Each action:
// 1. Validate preconditions
// 2. Apply changes atomically (reuse existing services)
// 3. Regenerate KeyViewer artifacts
// 4. Write status banner text
// 5. Trigger reload handshake

pub async fn handle_toggle_safe_mode(state: &AppState) -> Result<(), String>
pub async fn handle_next_preset(state: &AppState) -> Result<(), String>
pub async fn handle_prev_preset(state: &AppState) -> Result<(), String>
pub async fn handle_next_folder(state: &AppState) -> Result<(), String>
pub async fn handle_prev_folder(state: &AppState) -> Result<(), String>
pub fn generate_status_banner(safe_mode: bool, preset: &str, folder: Option<&str>, character: Option<&str>) -> String
pub fn send_reload_key(game_hwnd: HWND, key: &str) -> Result<(), String>
```

**Reuses:** `PrivacyManager::switch_mode` for safe toggle, `collections::apply::apply_collection` for preset switching.

---

#### [MODIFY] [mod.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/mod.rs)

Add `pub mod hotkeys;`

---

#### [MODIFY] [models.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/config/models.rs)

Add `HotkeyConfig` and `KeyViewerConfig` to `AppSettings`.

```rust
// New fields in AppSettings:
pub hotkeys: HotkeyConfig,
pub keyviewer: KeyViewerConfig,

// New structs:
pub struct HotkeyConfig {
    pub enabled: bool,
    pub safe_toggle: String,
    pub next_preset: String,
    pub prev_preset: String,
    pub next_folder: String,
    pub prev_folder: String,
    pub toggle_status: String,
    pub cooldown_ms: u64,
}

pub struct KeyViewerConfig {
    pub enabled: bool,
    pub toggle_key: String,        // Default: "H"
    pub ttl_seconds: f32,          // Default: 0.35
    pub min_hold_seconds: f32,     // Default: 0.20
    pub status_ttl_seconds: f32,   // Default: 3.0
    pub max_lines: u32,            // Default: 60
    pub max_bytes: u32,            // Default: 8192
    pub debug_overlay: bool,       // Default: false
    pub reload_key_override: Option<String>,
}
```

### Tests for Phase 4

#### [NEW] [hotkey_tests.rs](file:///e:/Dev/EMMM2NEW/src-tauri/src/services/hotkeys/tests/hotkey_tests.rs)

```
Test cases:
1. debounce_blocks_rapid — Second trigger within cooldown ignored
2. debounce_allows_after_cooldown — Trigger after cooldown proceeds
3. status_banner_safe_on — Banner shows "Safe: ON"
4. status_banner_preset — Banner shows preset name
5. status_banner_folder — Banner shows folder name
6. config_defaults — Default hotkey config has expected values
```

Run: `cargo test --lib hotkeys`

**Note:** Hotkey registration and keystroke sending are OS-level and cannot be tested in unit tests. Integration testing for Phase 4 requires manual verification with the game running.

---

## Phase 5: Frontend UI

#### [NEW] `src/features/settings/components/HotkeySettings.tsx`

Settings panel for configuring hotkeys, KeyViewer options, and resource pack management. Uses existing settings page patterns.

#### [MODIFY] `src/features/settings/` components

Add tab/section for KeyViewer & Hotkey configuration.

---

## Phase 6: Integration & E2E

Full pipeline wiring and end-to-end testing. Details TBD after Phases 1-5 complete.

---

## Verification Plan

### Automated Tests

Every phase has dedicated unit tests as described above. Run all with:

```bash
# All keyviewer backend tests
cargo test --lib keyviewer

# All hotkey backend tests
cargo test --lib hotkeys

# Full backend test suite (verify no regressions)
cargo test
```

### Manual Verification

> [!IMPORTANT]
> **The in-game overlay and hotkey functionality CANNOT be fully automated-tested.** The following require manual verification with a game running:

1. **KeyViewer overlay display** — Launch a supported game with 3DMigoto, ensure overlay appears when toggle key pressed, shows correct keybinds for detected character
2. **OS hotkeys** — Verify F5/F6/F7/F8 trigger correct actions while game is focused
3. **Reload handshake** — Verify reload key is sent to game or CTA shown
4. **Safe Mode toggle** — Toggle safe mode via F5 in-game, verify NSFW mods disappear after reload
5. **Preset switching** — Cycle presets via F6, verify workspace changes and banner shows new preset

**Recommendation:** I suggest we verify the pipeline end-to-end by generating sample output files and inspecting them manually, before attempting actual in-game testing. Each phase can be independently verified via its unit tests.
