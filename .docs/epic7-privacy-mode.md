# Epic 7: Privacy Mode (Master Mode Switcher)

**Focus:** Managing the exclusive transition between safe (SFW) and unsafe (NSFW) content modes. Uses SQLite for instant mass command execution and `info.json` for local data integrity, with an _Atomic Transaction_ guarantee.

## Dependencies

| Direction    | Epic      | Relationship                                                                                     |
| ------------ | --------- | ------------------------------------------------------------------------------------------------ |
| ⬆ Upstream   | Epic 5    | Uses mass toggle service for physical renames                                                    |
| ⬇ Downstream | Epic 3    | Sidebar reacts to `appStore.safeMode` (Zustand)                                                  |
| ⬇ Downstream | Epic 4    | Grid filters by `is_safe`                                                                        |
| ⬇ Downstream | Epic 8    | Collections respect `is_safe_context`                                                            |
| ⬇ Downstream | Epic 13   | Dashboard queries filter by Safe Mode                                                            |
| Owns         | Safe Mode | **This Epic is the SINGLE OWNER** of Safe Mode logic. E3/E4/E13 react to state, not redefine it. |

## Cross-Cutting Requirements

- **Dual Guard (Defense in Depth):**
  - **Frontend:** Zustand `appStore.safeMode`. When ON, React renders ONLY `is_safe` items. NSFW never in DOM.
  - **Backend:** SQL filter `AND is_safe = 1` added to all queries when `safe_mode_active = true`.
- **Default Mode:** App ALWAYS starts in SFW mode regardless of previous session.
- **PIN Reset:** If user forgets PIN: manually delete `pin_hash` from `config.json` (documented in Help section).
- **Watcher Suppression:** During bulk rename, use path suppression (TRD §3.5), NOT watcher pause/resume.
- **Operation Lock:** Mode switch acquires `OperationLock` (TRD §3.6).

---

## A. User Stories & Acceptance Criteria

### US-7.1: Master Mode One-Click Swap

**As a** user, **I want** a single main button to switch between "SFW Mode" and "NSFW Mode", **So that** the application physically activates relevant mods and deactivates the rest in a single fast transaction.

- **Acceptance Criteria:**
  - **Exclusive Toggle:** When one mode is active, the other categories must be hidden from the UI and physically `DISABLED` in the game folder.
  - **Hybrid Storage:** `last_status_active` status is stored in **SQLite** for rapid access, while the `is_safe` flag remains in the mod folder's `info.json` for portability.
  - **Atomic Operation:** The mode switching process is performed in a single sequence of commands (Single Transaction). If a file rename fails fatally, the entire operation is rolled back (Rollback) to prevent partial data corruption.

### US-7.2: Privacy Tagging & Auto-Classification

**As a** user, **I want** the system to automatically tag new mods based on keywords, **So that** I don't need to manually check every mod to determine its safety category.

- **Acceptance Criteria:**
  - **Keyword Match:** If a folder name contains keywords from `settings.safe_mode_keywords` (e.g., "Nude", "Hentai"), the mod is automatically tagged as `is_safe: false`.
  - **Manual Override:** Users can change the `is_safe` status in the UI, which automatically updates SQLite (`is_safe` column) and the `info.json` file in the related folder.

### US-7.3: Safe Mode Lock (Privacy Security)

**As a** user, **I want** to lock the Safe Mode button with a PIN, **So that** NSFW mode cannot be accidentally activated by others.

- **Acceptance Criteria:**
  - **PIN Protection:** If `settings.safe_mode_pin_enabled` is `true`, the system requests a 6-digit PIN before switching to NSFW mode.
  - **Rate Limiting:** If the PIN is incorrect 3 times, block input for 60 seconds.

---

## B. Technical Specifications (Atomic Rust Implementation)

### 1. Hybrid State Storage

The system splits data into two locations:

- **SQLite (Active State):** The `mods` table has `last_status_sfw` and `last_status_nsfw` (Boolean) columns. This acts as "Snapshot Memory".
- **`info.json` (Portable Metadata):** Stores `is_safe` (Boolean) inside the mod folder.

### 2. Atomic Switch Transaction (Code Specification)

Uses `sqlx::Transaction` to guarantee data consistency before physical operations.

```rust
use sqlx::{SqlitePool, Transaction};
use std::collections::HashMap;

struct PrivacyManager {
    db: SqlitePool,
}

impl PrivacyManager {
    // Main Function: Switch Mode
    async fn switch_to_sfw(&self) -> Result<(), AppError> {
        let mut tx = self.db.begin().await?;

        // 1. Snapshot current state (Active NSFW Mods)
        // Save their 'enabled' status to the 'last_status_nsfw' column
        sqlx::query!(
            "UPDATE mods SET last_status_nsfw = is_enabled WHERE is_safe = 0"
        ).execute(&mut tx).await?;

        // 2. Disable All NSFW Mods in Database
        sqlx::query!(
            "UPDATE mods SET is_enabled = 0 WHERE is_safe = 0"
        ).execute(&mut tx).await?;

        // 3. Restore SFW State (Re-enable mods previously active in SFW mode)
        sqlx::query!(
            "UPDATE mods SET is_enabled = last_status_sfw WHERE is_safe = 1"
        ).execute(&mut tx).await?;

        // 4. Commit DB Transaction
        tx.commit().await?;

        // 5. Physical Execution (Batch Rename)
        // Performed AFTER the DB is secure. If a crash occurs here, the DB is already correct,
        // just run 'Sync' upon restart.
        self.apply_physical_changes(Mode::SFW).await?;

        Ok(())
    }

    async fn apply_physical_changes(&self, target_mode: Mode) -> Result<(), AppError> {
        // Retrieve all mods from the DB
        let mods = self.get_all_mods().await?;

        for mod_data in mods {
            let should_be_enabled = mod_data.is_enabled;
            // Call rename logic from Epic 5
            super::core::update_mod_state(&mod_data.path, should_be_enabled)?;
        }
        Ok(())
    }
}
```

### 3. Edge Case Handling (Robustness)

- **Folder Missing:** If a target folder is missing during transition, the `update_mod_state` function will throw a `ModNotFoundError`. The system will catch this error, log it, but **continue** with the rest of the mod queue (Soft Fail).
- **Watchdog Suspension:** During the `apply_physical_changes` process, the Watchdog is temporarily disabled:
  ```rust
  watcher.pause()?;
  privacy_manager.switch_mode().await?;
  watcher.resume()?;
  ```

---

## C. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Enter Safe Mode**: Click "Safe Mode" → All NSFW mods renamed to `DISABLED` → UI hides them → "SFW Active" badge appears.
- [ ] **Exit Safe Mode**: Click "Disable Safe Mode" → Enter correct PIN → NSFW mods restored → UI shows them.
- [ ] **One-Click Swap**: Toggle Mode → 100+ mods update status (DB + Physical) in **< 2 seconds**.
- [ ] **State Memory**: Enable mod A in NSFW → Switch to SFW → Switch back to NSFW → Mod A restored to Enabled.
- [ ] **Default SFW**: App always starts in SFW mode, regardless of previous session state.

### 2. Negative Cases (Error Handling)

- [ ] **Wrong PIN**: Enter invalid PIN → Field shakes → Access denied → Retry count increments.
- [ ] **Lockout**: **5** wrong PIN attempts → Input disabled for 60 seconds (backend timer).
- [ ] **Missing Targets**: DB lists "Mod A" as NSFW but folder missing → Log warning, skip, finish rest.
- [ ] **Operation Lock**: User tries to toggle mods during mode switch → Toast: "Mode switch in progress. Please wait."

### 3. Edge Cases (Stability)

- [ ] **App Crash Mid-Transition**: Kill app during bulk rename → Restart → Startup Recovery detects DB vs Physical mismatch → Auto Fix.
- [ ] **External Modification**: Folder renamed in Explorer while Safe Mode active → Watcher catches changes → Updates DB without leaking NSFW.
- [ ] **Dual Guard Verified**: In SFW mode, NSFW items not in DOM (frontend) AND not returned by API (backend SQL filter).
- [ ] **Dashboard Safety**: E13 Dashboard only shows SFW-safe data when Safe Mode active.

### 4. Technical Metrics

- [ ] **Transition Speed**: Toggle mode for 500 mods completes in **< 2 seconds**.
- [ ] **Leak Proof**: Searching restricted keywords in Safe Mode returns **0 results**.
- [ ] **Accessibility**: Safe Mode toggle has ARIA label and keyboard shortcut.
