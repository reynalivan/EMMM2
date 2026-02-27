# Epic 5: Core Mod Management (Daily Operations)

**Focus:** Handling direct user interactions with mods (Toggle, Import, Bulk Action), ensuring synchronization of physical file status with the database, and automatically standardizing messy folder names.

## Dependencies

| Direction    | Epic    | Relationship                                             |
| ------------ | ------- | -------------------------------------------------------- |
| ⬆ Upstream   | Epic 2  | Uses Deep Matcher for Smart Import                       |
| ⬆ Upstream   | Epic 3  | Requires category context for "Enable Only This"         |
| ⬆ Upstream   | Epic 4  | Uses trash service for delete operations                 |
| ⬇ Downstream | Epic 7  | Provides mass toggle service for Safe Mode switch        |
| ⬇ Downstream | Epic 8  | Provides toggle service for Collection apply/undo        |
| ⬇ Downstream | Epic 10 | Provides toggle service for randomizer                   |
| References   | Epic 2  | Shader conflict detection (owned by E2, referenced here) |

## Cross-Cutting Requirements

- **Operation Lock:** All toggle/rename/import/delete operations acquire `OperationLock` (TRD §3.6). Concurrent requests return error toast.
- **TanStack Query:** After every operation, call `queryClient.invalidateQueries(['mods', gameId])`.
- **Archive Extraction:** Use `compress-tools` crate (per TRD) for ZIP, 7Z, RAR.
- **Undo Toast:** Toggle/Delete success toast includes "Undo" button (5s timeout). Clicking reverts the rename and DB state.
- **Bulk Progress:** Bulk operations show progress bar "Processing X/N..." with error collection summary.
- **Watcher Suppression:** Register paths in suppression set before rename (TRD §3.5).

---

## A. User Stories & Acceptance Criteria

### US-5.1: Toggle Enable/Disable (Atomic Switch)

**As a** user, **I want to** activate/deactivate mods with a single click, **So that** I can manage game loadouts without touching Windows Explorer.

- **Acceptance Criteria:**
  - **Pre-Flight Check:** Before renaming, the system **MUST** check if the physical folder still exists. If missing -> Display an Error & Trigger Auto-Refresh.
  - **Prefix Standardization:** When toggling (both Enable to Disable and vice versa), the system automatically corrects the prefix format.
    - _Bad Input:_ `disabled_Ayaka`, `DISABLED-Ayaka`, `Disabled Ayaka`.
    - _Standard Output:_ `DISABLED Ayaka` (Capital + Space) or `Ayaka` (if Enabled).
  - **Visual Feedback:** Toggle status in the UI changes instantly (Optimistic UI), then rolls back if the file operation fails.

### US-5.2: Smart Import (Drag, Drop, & Install)

**As a** user, **I want to** drop a zip file into the application, then have the application extract it, recognize its content, and move it to the active game folder.

- **Acceptance Criteria:**
  - **Input:** `.zip`, `.rar`, `.7z` files (Drag & Drop).
  - **Process:**
    1.  Extract to a Temp Folder.
    2.  Run **Intelligent Mod Scanning** (Epic 2) for identification.
    3.  **Move:** Move the extracted result to the active game's `/Mods` folder.

  - **Naming:** Use the scanned folder name (which is already clean). Default status after import: **Disabled**.

### US-5.3: "Enable This Only" (Single Skin Mode)

**As a** user, **I want** an "Enable Only This" option, **So that** other mods for the same character (e.g., Raiden) are automatically disabled, preventing texture conflicts/glitches.

- **Acceptance Criteria:**
  - **Scope:** Only applies to mods within the same **Character Category** (based on Master DB ID).
  - **Logic:**
    1.  Scan all active mods within the same physical object directory (represented by `folder_path`).
    2.  Disable all those mods by renaming them with the `DISABLED ` prefix on the filesystem.
    3.  Enable the selected target mod.

  - **Batch Operation:** This process must be atomic (one UI transaction).

### US-5.4: Manual Refresh & Sync

**As a** user, **I want** a refresh button, **So that** if I manually change a folder name in Explorer, the application can adjust its data.

- **Acceptance Criteria:**
  - **Action:** Refresh button on the Toolbar.
  - **Process:**
    1.  Re-scan the physical folders in the game directory (now integrated directly into `get_objects_cmd` as a self-healing mechanism).
    2.  Register new physical folders into the database.
    3.  Update status (Enabled/Disabled) for folders that have changed names on the filesystem.
    4.  Run **Prefix Standardization** on all folders detected as "Disabled" but with an incorrect format.

### US-5.5: Bulk Action Management

**As a** user, **I want to** select multiple mods at once to manage them.

- **Acceptance Criteria:**
  - **Multi-Select:** Checkboxes on table rows / Shift+Click.
  - **Actions:**
    - **Enable Selected:** Activate all selected mods.
    - **Disable Selected:** Deactivate all selected mods.
    - **Delete Selected:** Delete the physical folders (with confirmation).

### US-5.6: Duplicate Character Warning

**As a** careless user, **I want** the application to warn me if I try to activate 2 different mods for the character "Ganyu" at the same time, **So that** my game doesn't glitch or crash.

- **Acceptance Criteria:**
  - **Trigger:** When the user clicks Toggle Enable on Mod A.
  - **Logic:** Check if there is a Mod B (Disabled/Enabled) that has the same `master_object_id` as Mod A AND its status is currently **Enabled**.
  - **Action:**
    - If Count > 0: Display a **Warning Popup**.
    - "A mod for [Character Name] is already active! Disable the old one first?"
    - **Buttons:** "Force Enable" (Risky) or "Cancel" (Safe).

### US-5.7: Shader Conflict Warning (Notice Only)

**As a** user, **I want** a subtle notification if the mod I am activating has hash conflicts with another mod, **So that** I am aware of potential visual glitches.

- **Acceptance Criteria:**
  - **Scope:** Check `.txt` files in `ShaderFixes` and `hash = ...` in `.ini`.
  - **Alert Type:** Non-blocking Banner (Toast) "Shader Collision detected with [Mod Name]".
  - **Behavior:** Do not prevent the user; just provide information.

---

## B. Technical Specifications (Strict Logic)

### 1. Standardization & Rename Logic (The Fixer)

This function handles fixing "messy" folder names as per your request.

```rust
use std::path::{Path, PathBuf};
use regex::Regex;

fn standardize_prefix(folder_name: &str, target_state: bool) -> String {
    // Regex matches "disabled" variations at start (case insensitive)
    let re = Regex::new(r"(?i)^(disabled)[_\-\s]*").unwrap();

    // 1. Clean old prefix
    let clean_name = re.replace(folder_name, "").trim().to_string();

    // 2. Return new name based on state
    if target_state {
        clean_name // ENABLE (Remove prefix)
    } else {
        format!("DISABLED {}", clean_name) // DISABLE (Add standard prefix)
    }
}

fn safe_rename(base_path: &Path, old_name: &str, new_name: &str) -> Result<String, String> {
    let old_path = base_path.join(old_name);
    let new_path = base_path.join(new_name);

    // Pre-check: Source must exist
    if !old_path.exists() {
        return Err(format!("Source folder not found: {}", old_name));
    }

    // Collision check: Destination must not exist (unless differing only by case/renaming same file)
    if new_path.exists() && old_name.to_lowercase() != new_name.to_lowercase() {
        return Err(format!("Destination folder already exists: {}", new_name));
    }

    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(new_name.to_string())
}
```

### 2. Logic "Enable Only This" (Conflict Resolver)

```rust
use sqlx::{SqlitePool, Row};

async fn enable_only_this_mod(pool: &SqlitePool, target_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx = pool.begin().await?;

    // 1. Get Target Info & Category
    let target = sqlx::query("SELECT path, category_id FROM mods WHERE id = ?")
        .bind(target_id)
        .fetch_one(&mut *tx).await?;
    let category_id: String = target.get("category_id");
    let target_path: String = target.get("path");

    // 2. Find Conflicts (Active mods in same category)
    let conflicts = sqlx::query("SELECT id, path FROM mods WHERE category_id = ? AND is_enabled = 1 AND id != ?")
        .bind(&category_id)
        .bind(target_id)
        .fetch_all(&mut *tx).await?;

    // 3. Disable Conflicts
    for row in conflicts {
        let path: String = row.get("path");
        let name = Path::new(&path).file_name().unwrap().to_str().unwrap();
        let new_name = standardize_prefix(name, false); // Disabled

        let parent = Path::new(&path).parent().unwrap();
        safe_rename(parent, name, &new_name)?;

        // Update DB
        sqlx::query("UPDATE mods SET is_enabled = 0, path = ? WHERE id = ?")
            .bind(parent.join(&new_name).to_str().unwrap())
            .bind(row.get::<String, _>("id"))
            .execute(&mut *tx).await?;
    }

    // 4. Enable Target
    let target_name = Path::new(&target_path).file_name().unwrap().to_str().unwrap();
    let target_new_name = standardize_prefix(target_name, true); // Enabled
    let target_parent = Path::new(&target_path).parent().unwrap();

    safe_rename(target_parent, target_name, &target_new_name)?;

    sqlx::query("UPDATE mods SET is_enabled = 1, path = ? WHERE id = ?")
        .bind(target_parent.join(&target_new_name).to_str().unwrap())
        .bind(target_id)
        .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}
```

### 3. Duplicate Character Logic

Prevents broken graphics due to multi-mod overlapping.

```rust
async fn check_duplicate_enabled(pool: &SqlitePool, target_mod_id: &str) -> Result<bool, sqlx::Error> {
    // 1. Get Master Object ID from target
    let target = sqlx::query("SELECT master_object_id FROM mods WHERE id = ?")
        .bind(target_mod_id)
        .fetch_optional(pool).await?;

    if let Some(row) = target {
        let master_id: Option<String> = row.get("master_object_id");
        if master_id.is_none() { return Ok(false); } // Not a character mod

        // 2. Count other enabled mods with same Master ID
        let count: i64 = sqlx::query_scalar(r#"
            SELECT count(*) FROM mods
            WHERE master_object_id = ?
            AND is_enabled = 1
            AND id != ?
        "#)
        .bind(master_id)
        .bind(target_mod_id)
        .fetch_one(pool).await?;

        Ok(count > 0)
    } else {
        Ok(false)
    }
}
```

### 4. Smart Import Flow (Integration)

1.  **Extract:** Unzip to `./temp_extract`.
2.  **Epic 2 Scan:** Call the `DeepMatcher` module for the extracted folder.
    - _Result:_ `{ "name": "Ayaka Summer", "category": "Ayaka", "confidence": "High" }`

3.  **Sanitize:** Ensure the folder name is clean.
4.  **Install:** Move the folder from `./temp_extract/Ayaka Summer` to `GAME_ROOT/Mods/DISABLED Ayaka Summer`.
5.  **DB Record:** Insert new mod data into SQLite with a Disabled status.

---

## C. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Toggle Enable**: Click "Enable" on `DISABLED Raiden` → Renames to `Raiden` → UI badge turns Green.
- [ ] **Toggle Disable**: Click "Disable" on `Raiden` → Renames to `DISABLED Raiden` → UI badge turns Grey.
- [ ] **Smart Import**: Drag `Raiden.zip` to App → Extracted → Scanned as "Raiden" → Moved to `/Mods/DISABLED Raiden`.
- [ ] **Enable Only This**: Select `Raiden B` (with `Raiden A` active) → Click "Enable Only This" → `A` Disabled, `B` Enabled automatically.
- [ ] **Undo Toggle**: After toggle, toast shows "Undo" button → Click within 5s → State reverts successfully.
- [ ] **Bulk Archive Import**: Drop 3 zip files at once → queued extraction → all processed sequentially → summary modal.

### 2. Negative Cases (Error Handling)

- [ ] **File Locked**: Mod file matches `d3dx.ini` (Locked by Game) → Toggle fails → Show "Access Denied" Toast → UI reverts state.
- [ ] **Import Corrupt**: User drags broken zip → Import fails → Temporary files cleaned up → Error toast displayed.
- [ ] **Duplicate Warning**: User tries enabling 2nd Ganyu mod → Alert "Duplicate Character Active" blocks action (unless Forced).
- [ ] **Operation Lock**: User clicks toggle while another toggle runs → Toast: "Operation in progress. Please wait." (TRD §3.6).

### 3. Edge Cases (Stability)

- [ ] **Bad Prefix Fix**: Toggle `disabled-ayaka` → Becomes `Ayaka`. Toggle back → Becomes `DISABLED Ayaka` (Standardized).
- [ ] **Extended Regex**: System also handles `dis_ayaka`, `DISABLE ayaka`, `Disable-Ayaka` variants.
- [ ] **External Deletion**: Folder deleted in Explorer → Click Toggle in App → System catches `FileNotFound`, shows error, and auto-refreshes list.
- [ ] **Race Condition**: User clicks Toggle on 5 mods rapidly → Operation lock processes them sequentially without data corruption.
- [ ] **Watcher Suppression**: In-app toggle does NOT trigger watcher event (TRD §3.5).

### 4. Technical Metrics

- [ ] **Toggle Latency**: Rename operation + UI reflection take **< 50ms**.
- [ ] **Import Speed**: Extraction and Scan of 500MB zip completes in **< 5 seconds** (using `compress-tools`).
- [ ] **Accessibility**: All toggle buttons have ARIA labels and keyboard support.
