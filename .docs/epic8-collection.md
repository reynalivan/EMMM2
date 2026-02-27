# Epic 8: Virtual Collections (Loadout Presets)

**Focus:** Providing users with the ability to create, manage, and apply mass modification packages (_Presets_) that are sensitive to game context and safety modes (SFW/NSFW), equipped with a snapshot-based "Undo" security system.

## Dependencies

| Direction  | Epic   | Relationship                                         |
| ---------- | ------ | ---------------------------------------------------- |
| ⬆ Upstream | Epic 5 | Uses toggle service for Apply/Undo                   |
| ⬆ Upstream | Epic 7 | Respects `is_safe_context` + Safe Mode state         |
| ⬆ Upstream | Epic 4 | Uses trash service for deleted members               |
| References | Epic 2 | Shader conflict detection during apply (owned by E2) |

## Cross-Cutting Requirements

- **DB Table:** Uses `collection_items` (renamed from `collection_members` per TRD alignment).
- **Columns:** `collections` table has `is_safe_context BOOLEAN DEFAULT 0` (added per TRD update).
- **Operation Lock:** Apply/Undo acquire `OperationLock` (TRD §3.6).
- **TanStack Query:** After apply/undo, invalidate `['mods', gameId]` and `['collections', gameId]`.
- **Snapshot:** Undo state stored as JSON array of `{ mod_id, previous_status }` in memory (max 1 undo level).

---

## 1. User Stories & Acceptance Criteria

### US-8.1: Loadout Creation & Context Sensitivity

**As a** user, **I want to** create mod collections (e.g., "Beach Party"), **So that** I can activate a specific group of mods across various characters at once with just one click.

- **Acceptance Criteria:**
  - **Context Isolation:** Collections are unique per game (A GIMI preset will not appear in SRMI).
  - **Safety Awareness:** Collections are separated based on Safe Mode status. Presets created in NSFW mode **WILL NOT** be visible when Safe Mode is ON (SFW) to prevent content leakage.
  - **Membership:** A single mod can be a member of multiple collections simultaneously (Many-to-Many).

### US-8.2: Mass Preset Activation (Atmospheric Swap)

**As a** user, **I want to** activate a preset, **So that** the application automatically activates the mods listed in that preset and deactivates other mods in the same object categories.

- **Acceptance Criteria:**
  - **Smart Conflict Resolution:**
    - If a Preset activates "Raiden Bikini", then the currently active "Raiden Default" mod must be **automatically disabled**.
    - Other character mods _not touched_ by the preset (e.g., Zhongli) remain in their original status (unchanged).
  - **Physical Transformation:** When a preset is activated, the system performs a parallel _bulk rename_ of physical folders using `standardize_prefix` logic.

### US-8.3: Smart Tracing & Healing

**As a** user, **I want** the system to still recognize collection members even if their physical folders are renamed, **So that** preset relationships are not broken.

- **Acceptance Criteria:**
  - **Double ID Tracking:** The system stores the relative path AND the mod's unique hash. If the path changes, the system attempts to _re-link_ based on the hash.
  - **Missing Handler:** If a physical mod is completely missing, the system provides a notification: _"Skipping missing mod: [Mod Name]"_ and continues the rest of the process.

### US-8.4: Safe Apply Confirmation & Undo (Cheat Death)

**As a** user, **I want to** be able to cancel the application of a preset (Undo) if the result is not to my liking, **So that** I can experiment without fear.

- **Acceptance Criteria:**
  - **Auto-Snapshot:** Before a preset is applied, the system **MUST** save a list of IDs for all currently _Enabled_ mods into memory/timeline.
  - **Undo Capability:** The success toast has an "Undo" button.
  - **Restoration:** Pressing "Undo" restores the Enable/Disable status of all mods exactly to their condition before the preset was applied in **< 1 second**.

---

## 2. Technical Specifications (Rust/Tauri Implementation)

### A. Database Schema (SQLite Integration)

Relational structure for storing collections and their members.

```sql
-- Collections Table
CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    game_id TEXT NOT NULL,
    is_safe_context BOOLEAN NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Members Table
CREATE TABLE collection_members (
    collection_id TEXT,
    mod_id TEXT, -- Relation to mods table
    mod_path TEXT, -- Fallback if ID changes (re-import)
    PRIMARY KEY (collection_id, mod_id),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
);
```

### B. Rust Logic (Snapshot & Apply)

Rust implementation for efficient Snapshot and Undo features.

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone)]
struct ModStateSnapshot {
    timestamp: u64,
    enabled_mod_ids: Vec<String>, // List of active mod IDs BEFORE the preset
}

struct CollectionManager {
    db: SqlitePool,
    undo_stack: Vec<ModStateSnapshot>, // In-Memory Stack
}

impl CollectionManager {
    // 1. Apply Preset Strategy
    async fn apply_preset(&mut self, collection_id: &str) -> Result<(), AppError> {
        let mut tx = self.db.begin().await?;

        // A. Capture Snapshot (For Undo)
        let current_active = sqlx::query!("SELECT id FROM mods WHERE is_enabled = 1")
            .fetch_all(&mut tx).await?;

        self.undo_stack.push(ModStateSnapshot {
            timestamp: now(),
            enabled_mod_ids: current_active.iter().map(|r| r.id.clone()).collect(),
        });

        // B. Get Target Mods
        let targets = sqlx::query!(
            "SELECT mod_id FROM collection_members WHERE collection_id = ?",
            collection_id
        ).fetch_all(&mut tx).await?;

        // C. Logic: Disable Conflicts, Enable Targets
        for target in targets {
            // Find conflicts (same char_id, currently enabled)
            let conflict = get_conflict(&mut tx, &target.mod_id).await?;
            if let Some(c) = conflict {
                disable_mod(&mut tx, &c).await?;
            }
            enable_mod(&mut tx, &target.mod_id).await?;
        }

        tx.commit().await?;

        // D. Apply Physical Changes (Bulk)
        self.sync_physical().await?;

        Ok(())
    }

    // 2. Undo Logic
    async fn undo_last_action(&mut self) -> Result<(), AppError> {
        if let Some(snapshot) = self.undo_stack.pop() {
             let mut tx = self.db.begin().await?;

             // Disable ALL currently enabled mods
             sqlx::query!("UPDATE mods SET is_enabled = 0").execute(&mut tx).await?;

             // Re-enable mods from snapshot
             for mod_id in snapshot.enabled_mod_ids {
                 sqlx::query!("UPDATE mods SET is_enabled = 1 WHERE id = ?", mod_id)
                     .execute(&mut tx).await?;
             }

             tx.commit().await?;
             self.sync_physical().await?;
        }
        Ok(())
    }
}
```

### C. Metadata Portability (`info.json`)

To ensure collection identity is preserved when a mod folder is physically moved:

- The system will write the collection name into the `preset_name` field in the mod folder's local `info.json` file.
- If a mod is deleted, the data in the `collection_items` DB will become _orphaned_ and will be cleaned up during a weekly _Maintenance Task_ (E11).

---

## 3. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Create Collection**: Select 5 mods → "Create Preset" → Name "Abyss Team" → Preset saved to DB.
- [ ] **Apply Preset**: Click "Apply Abyss Team" → Confirm Modal → Current mods disabled → Preset mods enabled.
- [ ] **Undo Function**: Click "Undo" on Success Toast → Mods revert to exact previous state instantly.

### 2. Negative Cases (Error Handling)

- [ ] **Missing Member**: Preset contains "Mod X" which was deleted → Warning "Mod X not found" → Applies remaining valid mods.
- [ ] **Conflict Handling**: Preset enables "Raiden A" but "Raiden B" is active → System auto-disables "Raiden B".
- [ ] **Context Mismatch**: Try to apply "Genshin" preset while in "Star Rail" → Action blocked.
- [ ] **Operation Lock**: User tries apply while another operation runs → Toast: "Operation in progress."

### 3. Edge Cases (Stability)

- [ ] **Double Apply**: Click "Apply" twice rapidly → Operation lock prevents duplicate execution.
- [ ] **Mixed Safety**: Preset contains NSFW mods → Apply in Safe Mode → Warning: "Contains NSFW mods. Switch mode or skip.".
- [ ] **Empty Collection**: Collection with 0 items → Apply button disabled. Delete button still works.

### 4. Technical Metrics

- [ ] **Apply Speed**: Activation of 50-mod preset takes **< 100ms** (DB) and **< 2s** (Disk I/O).
- [ ] **Snapshot Size**: Undo history state JSON < 10KB.
- [ ] **Accessibility**: All collection buttons have ARIA labels. Keyboard shortcut for Apply.
