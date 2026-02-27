# Epic 2: Intelligent Mod Scanning & Organization (Finalized v3.0)

**Focus:** Implementation of the application's "Brain" that handles archive detection, deterministic staged matching (Quick/FullScoring), data normalization, and a Review interface that gives full control to the user.

## Dependencies

| Direction    | Epic   | Relationship                                   |
| ------------ | ------ | ---------------------------------------------- |
| ⬆ Upstream   | Epic 1 | Requires `games.path` for scan source          |
| ⬇ Downstream | Epic 3 | Provides matched `object_type` for sidebar     |
| ⬇ Downstream | Epic 4 | Provides `mods` table data for grid display    |
| ⬇ Downstream | Epic 5 | Provides Smart Import matching pipeline        |
| ⬇ Downstream | Epic 9 | Shares file walker and normalization utilities |

## Cross-Cutting Requirements

- **Archive Extraction:** Uses `zip` (v2, AES), `sevenz-rust` (v0.6), and `rar` (v0.4) crates for ZIP, 7Z, RAR support with smart flattening.
- **Fuzzy Compatibility:** Keep `strsim::normalized_levenshtein` available only for legacy compatibility tooling; staged runtime matching does not execute fuzzy fallback.
- **File Watcher:** Use `notify` crate v6+ with `RecommendedWatcher`. See TRD §3.5 for suppression strategy.
- **Review Table:** Use `TanStack Table` for headless data grid in Review UI.
- **Cache:** After scan completes, call `queryClient.invalidateQueries(['mods'])` via TanStack Query.
- **Shader Conflict Detection:** This Epic is the **single owner** of hash conflict logic. E5 and E10 reference this service, not redefine it.

---

## A. User Stories & Acceptance Criteria

### US-2.1: Archive Detection & Extraction Manager

**As a** user, **I want** the application to notify me if there are zip/rar files in my mod folder, and let me choose which ones to extract.

- **Acceptance Criteria:**
  - **Scan:** Detect `.zip`, `.rar`, `.7z` files in the root source folder.
  - **UI Confirmation:** Display a list of found archives with Checkboxes (Default: Checked).
  - **Action:** "Extract Selected" button.
  - **Logic:**
    - Extract to a new folder with the same name as the archive file.

  - **Logic (Smart Extraction):**
    - **Deep Folder Flattening**: If the zip contains `Single Folder/Mods/...`, the system automatically moves the `Mods` content to the extraction result's root (avoiding _nested folders_).
    - **Pre-Extraction Analysis**: Scan the archive up to a depth of 5 levels. If no `.ini` file is found, provide a `has_ini_warning` in the confirmation dialog.
    - Extract to a new folder with the same name as the archive file.
    - The original archive file is moved to the `.archive_backup` folder.
    - **Trigger Zones**: Archive extraction should also trigger when archives are dropped via Drag & Drop onto the auto-organize Sidebar (ObjectList) or the Folder Grid (main area).

### US-2.X: Instant Startup (Filesystem Source of Truth)

**As a** user with thousands of mods, **I want** the application to treat my folder structure as the absolute source of truth, reading data directly from the disk.

- **Acceptance Criteria:**
  - **Filesystem First**: The application reads the `.ini` and folder states directly from the filesystem (or a highly synced cache mirror representation) such that manual explorer changes are the primary source.
  - **Ready Indicator**: Display a "System Ready" indicator as soon as the initial UI data is processed.

### US-2.Y: Background Sync (Watchdog)

**As a** user who actively manages files in Explorer, **I want** the application to automatically synchronize if I rename or delete a folder outside the application.

- **Acceptance Criteria:**
  - **Real-time Update**: The UI refreshes the mod list automatically when file system events (Rename/Delete/Move/Create) occur.
  - **Debounce Logic**: Prevents event spam (e.g., during mass copy-paste) with a 500ms delay before triggering a refresh.
  - **Self-Awareness**: The Watchdog must recognize changes made by the application itself (to avoid refresh loops).

### US-2.Z: Shader & Buffer Conflict Notice (Non-Blocking)

**As a** user, **I want to** know if two mods are changing the same shader hash, **So that** I understand why one mod might not appear (glitch), without being prevented from still using them.

- **Acceptance Criteria:**
  - **Trigger:** During the Scan process or Mod Toggle.
  - **Detection:** Analyze `.ini` files for `[TextureOverride...]` with the same `hash = xxxx` on active mods.
  - **Notice:** Display a "Toast" or small warning icon: "Conflict detected with Mod X (Hash 1234)".
  - **Action:** The user is still allowed to activate both mods (Risk accepted).

### US-2.2: The Deep Matcher Pipeline (The Brain)

**As the** system, **I must** identify mod categories (Character, Weapon, UI, etc.) with an implemented staged deterministic pipeline and safe review-first outcomes.

- **Acceptance Criteria:**
  - **Scope:** Matching the source folder against the **Master DB** (combination of `db_char.json` and `db_other.json`).
  - **Tag Usage:** The `tags` field in the DB must be considered as an **Alias** equivalent to `name`.
  - **Modes:**
    - `Quick`: fast staged evaluation with shallow budgets.
    - `FullScoring`: deeper staged evaluation with broader content budgets.
  - **Staged Order (Implemented):**
    1. Hash overlap.
    2. Strict alias (folder tokens).
    3. **F3A:** Deep content substring pass A (subfolder/file stems).
    4. **F3B:** Deep content substring pass B (INI-derived strings).
    5. Alias recheck (post-expansion).
    6. Weighted token overlap (IDF-aware).
    7. Direct-name support.
    8. **AI Rerank (Optional):** Runs on NeedsReview.
    9. **Mechanical Rerank:** Active fallback for NeedsReview. Integrates GameBanana API v11 data (Mod Name, RootCategory, Description Keywords, File Stems) to decisively boost or penalize.
    10. **F9 Root Folder Rescue:** Last resort fallback for NoMatch.
  - **Statuses (Implemented):** `AutoMatched`, `NeedsReview`, `NoMatch`.
  - **Auto-Accept Rules:** `AutoMatched` is allowed only when stage threshold + margin pass and primary evidence gate passes.
  - **Non-Auto Rules:** Unmatched mods (`NeedsReview` and `NoMatch`) are auto-linked to a generic "Other" object (matching their folder name). They enter the grid UI immediately but remain candidates for user-driven Auto Organize later.
  - **Direct-Name Constraint:** Direct-name support cannot auto-match by itself.
  - **Optional AI Rerank:** Default OFF, runs only on `NeedsReview`, and can promote only if rerank threshold + margin gate passes.

### US-2.3: Review & Organize UI (Rich Interface)

**As a** user, **I want** an interactive table to review scan results, see thumbnails, and perform bulk edits before saving.

- **Acceptance Criteria:**
  - **Thumbnail System:** Display mod preview images with `preview*` name priority and _Lazy Loading_.
  - **Bulk Actions:** Checkbox on each row -> Action Bar (Move to Category X, Toggle Disable, Delete).
  - **Inline Edit:** The source folder name can be renamed directly in the table (clicking the pencil icon/double click).
  - **Open Folder:** Right-click/icon button to open the folder location in Windows Explorer.
  - **Override:** Users can change even "High Confidence" items' categories.
  - **Prefix Normalization:** In the UI, display folder names as if they already have the `DISABLED` prefix (if not already present), but the physical rename only happens when the **Confirm** button is pressed.

---

## B. Technical Specifications (No Gray Area)

### 1. Preprocessing Logic (Text Normalization)

Every string (Folder Name & File Name) must pass through this function before entering the Pipeline.

```rust
use deunicode::deunicode;
use regex::Regex;
use std::collections::HashSet;

fn preprocess_text(text: &str) -> HashSet<String> {
    // 1. Non-Latin Character Transliteration (Korean/Japanese/Chinese -> Latin)
    let text_latin = deunicode(text);
    // 2. Regex replace non-alphanumeric symbols with spaces
    let re = Regex::new(r"[^a-zA-Z0-9\s]").unwrap();
    let text_clean = re.replace_all(&text_latin, " ");
    // 3. Lowercase & Split & Insert to Set
    text_clean.to_lowercase().split_whitespace().map(|s| s.to_string()).collect()
}
```

### 2. Matching Pipeline Logic

**Resources:**

- `Master DB`: List of objects from `db_char` (Albedo, etc.) and `db_other` (Weapon, UI, etc.).
- `Aliases`: Combination of `name` and `tags` array from each DB object.
- `Indexes`: Deterministic token/hash indexes (seed + replenish candidate pool before scoring).

#### **Pipeline A: Quick Mode (Staged Fast Path)**

- **Intent:** Fast deterministic pass for large scans.
- **Stages (Q1-Q5):**
  1. **Q1 Hash Sniff:** Parse budgeted root INI hashes and score hash overlap.
  2. **Q2 Alias Strict:** Require full alias token presence (from normalized alias/tag tokens).
  3. **Q3 L3-lite Deep:** Use shallow deep signals from subfolder/file stems and INI section/content tokens.
  4. **Q4 Token Overlap:** Apply token-intersection score against entry tokens.
  5. **Q5 Direct-Name Support:** Add small ranking boosts only (no dedicated auto-accept stage).
- **Budget Profile:** Quick budgets are intentionally shallow and deterministic (small INI count/bytes, shallow depth).

#### **Pipeline B: FullScoring Mode (Staged Accurate Path)**

- **Intent:** Higher-accuracy deterministic pass with deeper budgets.
- **Stages (F1-F9 Pipeline):**
  1. **Hash Scan:** Parse recursive INI hashes and score quality-weighted hash overlap. (Acceptance Gate)
  2. **Alias Strict (Early):** Evaluate strict alias against folder tokens. (Acceptance Gate)
  3. **F3A Substring Pass A:** Substring matching over file stems and subfolder names. (Acceptance Gate)
  4. **Deep Token Overlap (Legacy - Skipped):** Bypassed in favor of later stages.
  5. **F3B Substring Pass B:** Substring matching over INI-derived strings (section headers, path-like values). (Acceptance Gate)
  6. **Alias Re-check:** Re-apply strict alias after deep/INI token expansion. (Acceptance Gate)
  7. **Weighted Token Overlap:** Apply IDF-aware token overlap on aggregated tokens. (Acceptance Gate)
  8. **Direct-Name Support:** Add supporting score only; cannot auto-match alone.
  9. **AI Rerank (Optional):** Trait-based LLM reranking for `NeedsReview` candidates.
  10. **Mechanical Rerank (GameBanana Supplement):** Fast point-based rerank for `NeedsReview`. Extracts `mod_name`, `root_category`, `description_keywords`, and `file_stems` from **GameBanana API v11** to provide massive exact-match bonuses and punishing mismatch penalties.
  11. **F9 Root Folder Rescue:** Last-resort fallback that runs purely on normalized root folder name if status is `NoMatch`.
- **Budget Profile:** FullScoring uses deeper deterministic limits (more INI files/bytes and deeper recursion) while preserving stable ordering/caps.

#### **Pipeline C: Acceptance, Status, and Review Finalization Gates**

- **Deterministic Ranking:** Candidates sort by score desc, then name asc, then entry_id asc.
- **Stage Auto-Accept Gate:** A stage can return `AutoMatched` only if:
  - threshold and margin pass, and
  - primary evidence gate passes (hash overlap, strict alias, or deep/INI primary evidence).
- **Ambiguity Controls:** Margin conflicts, ultra-close top candidates, pack-style mixed strong signals, and negative evidence penalties force review-safe outcomes.
- **Finalization:**
  - If a stage is accepted -> `AutoMatched`.
  - Else if best score meets review floor -> `NeedsReview` with deterministic top-k.
  - Else -> `NoMatch`.
- **Review Behavior:** `NeedsReview` and `NoMatch` are not auto-applied; user review finalization is required.

#### **Pipeline D: Optional AI Rerank + Legacy Fuzzy Compatibility**

- **AI Rerank (Optional):**
  - Default OFF.
  - Runs only when staged result is `NeedsReview`.
  - Promotion to `AutoMatched` requires rerank threshold and rerank margin gates.
  - Rerank is cacheable on signal fingerprint + DB version.
- **Fuzzy Compatibility Note:** Fuzzy matching is retained only for legacy compatibility pathways and is inactive in the staged runtime pipeline.

### 3. Skin/Variant Resolver Logic (Post-Match)

Only runs if `object_type == "Character"`.

- **Trigger Keywords:** Take from `db.official_skins` (if exist).
- **Logic:** Check if `folder_tokens` contains a skin trigger keyword.
- **Result:** Set `detected_skin` = Skin Name.

### 4. Thumbnail Crawler Logic (Lazy Load)

Image search function for UI Review.

```rust
use walkdir::WalkDir;
use std::path::{Path, PathBuf};

async fn find_thumbnail(mod_path: &Path) -> PathBuf {
    let valid_exts = ["png", "jpg", "jpeg", "webp", "gif"];

    // Priority 1: Check root for "preview.*"
    let root_files = std::fs::read_dir(mod_path).unwrap();
    for entry in root_files.flatten() {
        let path = entry.path();
        if let Some(stem) = path.file_stem() {
            if stem.to_string_lossy().to_lowercase().starts_with("preview") &&
               valid_exts.contains(&path.extension().unwrap_or_default().to_str().unwrap_or("")) {
                return path;
            }
        }
    }

    // Priority 2: Any image file in root
    for entry in std::fs::read_dir(mod_path).unwrap().flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if valid_exts.contains(&ext.to_str().unwrap_or("").to_lowercase().as_str()) {
                return path;
            }
        }
    }

    // Priority 3: Scan 1 level deep
    // Using WalkDir with max_depth(2)
    for entry in WalkDir::new(mod_path).max_depth(2).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if valid_exts.contains(&ext.to_str().unwrap_or("").to_lowercase().as_str()) {
                return path.to_path_buf();
            }
        }
    }

    // Fallback
    PathBuf::from("assets/placeholder.png")
}
```

### 5. Duplicate & Overwrite Handling Strategy

Prevents overwriting data without user approval.

- **Step 1:** Determine Destination Path (e.g., `../Raiden Shogun/DISABLED raiden32114`).
- **Step 2:** Check `os.path.exists(destination_path)`.
- **Step 3:** If Folder Already Exists:
  - **Action:** Mark as **DUPLICATE** in the Preview Table, then leave user to resolve follow-up actions like moving to general or renaming.
  - _Automated Rename_ (Optional Config): Add a unique suffix, e.g., `.../DISABLED raiden32114 (1)`.
- **Step 4:** If it doesn't exist yet -> status **READY TO MOVE**.

### 6. Stable Identification Strategy

- **Stable Identifiers**:
  - Do not use _Absolute Path_ as the database ID.
  - Use **SHA1 Hash** of the _Relative Path_ (e.g., `Mods/Raiden/ModA`) as the unique `id`.
  - _Benefit:_ Users can move the entire game folder to another drive (E: to D:) without breaking the database/mod history.

### 6. Review & Execution Flow

#### **Preview Table Data Structure**

Display a validation table before execution.

| No  | Source Folder (Short) | Destination Folder (Short)      | Matching Details              | Status               |
| :-- | :-------------------- | :------------------------------ | :---------------------------- | :------------------- |
| 1   | `.../ayaka-mods`      | `.../Ayaka/DISABLED ayaka-mods` | 90% Match (Name)              | Ready                |
| 2   | `.../abi`             | `.../Lumine/DISABLED abi`       | Matched via File `lumine.ini` | Ready                |
| 3   | `.../raiden_v2`       | `.../Raiden/DISABLED raiden_v2` | 85% Match (Name)              | **Skipped (Exists)** |

#### **Post-Process Reporting**

After the "Move Selected" button is pressed and the process is complete:

- **Summary Modal:**
  - ✅ **Moved:** 15 folders.
  - ⏭️ **Skipped:** 2 folders (Exists/Duplicate).
  - ⚠️ **Failed:** 0 folders (Permission/IO Error).
- **Details:** Option to view detailed logs of which files succeeded/failed.

---

## C. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Archive Extraction**: System identifies `.zip/.rar` files → UI prompts user → Extraction successful → Folder created → Archive moved to backup.
- [ ] **Smart Flattening**: Extraction of `Archive.zip` containing `Root/Mods/Target` results in `.../Target` (intermediate folders removed).
- [ ] **Staged Auto-Accept (Hash/Alias)**: Folder with strong hash/strict-alias evidence can resolve to `AutoMatched` through staged acceptance gates.
- [ ] **Staged Deep Recovery (Q3/F3)**: Folder named `Unknown 123` containing `Raiden.ini` is resolved deterministically to `AutoMatched` or `NeedsReview` based on acceptance gates.
- [ ] **Direct-Name Support Guard**: Direct-name-only signals never auto-match by themselves.
- [ ] **Legacy Fuzzy Compatibility**: Fuzzy fallback is inactive in staged runtime; unresolved typo-only cases remain review-driven.
- [x] **Conflict Notice**: System detects 2 enabled mods sharing `IB Hash 1234` → Displays non-intrusive "Conflict Detected" toast. (`ConflictToast` + `useActiveConflicts` hook)
- [x] **Scan Progress**: Progress bar shows "X/Y folders scanned" with ETA (`elapsed_ms` field) and Cancel button (`cancel_scan_cmd`).

### 2. Negative Cases (Error Handling)

- [ ] **Corrupt Archive**: Extraction meets a corrupted zip → System logs error, highlights item in Red, and continues processing next item (No Crash).
- [ ] **Permission Denied**: Destination folder is read-only → Move operation fails → System shows "Access Denied" error for that specific item.
- [ ] **No Match Found**: Folder `Random Stuff` matches nothing → Auto-linked to generic "Other" object with the folder's name (Not filtered out).
- [ ] **Missing Ini/Dll**: Deep scan finds no recognizable files → Returns "Unknown" status.
- [ ] **Disk Full**: If disk space < 2× archive size → abort extraction with error "Insufficient disk space".
- [x] **Scan Cancelled**: User clicks Cancel mid-scan → partial results cleaned up → DB not updated. (`cancel_scan_cmd` implemented)

### 3. Edge Cases (Stability)

- [ ] **Duplicate Destination**: Destination `.../Raiden/Mod A` already exists. System marks as "Duplicate" in Review Table and prevents overwrite until user resolves (Rename/Skip).
- [ ] **Non-ASCII Paths**: System handles folders with CJK (Chinese/Japanese/Korean) characters without encoding errors during Move/Rename.
- [ ] **Deep Nesting**: Scanner handles folders nested > 5 levels deep without StackOverflow or hang.
- [ ] **Zero-Byte Files**: Scanner encounters 0kb `.ini` files → Ignored gracefully.
- [ ] **File Watcher Suppression**: In-app move operations do NOT trigger watcher re-scan (per TRD §3.5).

### 4. Technical Metrics

- [ ] **Scan Speed**: Deep scan of **10GB / 500 folders** completes in **< 10 seconds** (SSD).
- [ ] **Memory Integration**: RAM usage during full scan stays below **500MB**.
- [ ] **Watchdog Latency**: External file rename reflects in UI in **< 1 second**.
- [ ] **Cancellation**: `tokio::CancellationToken` stops scan within **< 200ms** after Cancel click.
