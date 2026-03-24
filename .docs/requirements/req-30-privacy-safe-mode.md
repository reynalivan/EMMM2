# Epic 30: Privacy & Safe Mode

## 1. Executive Summary

- **Problem Statement**: Users manage mods with varying content sensitivity. Opening the app on stream or in public risks displaying NSFW thumbnails and names. A fast, trustworthy privacy layer is required to prevent accidental exposure.
- **Proposed Solution**: A global Safe Mode toggle (shield icon) backed by `safe_mode` in the backend `AppSettings` (SQLite). Features include: visual masking (blur + obfuscated names) for out-of-corridor mods, auto-classification based on folder name keywords, strict Dual Guard isolation (frontend masking + backend exclusion from queries/counts), and an Argon2-hashed PIN gate.
- **Success Criteria**:
  - [x] Safe Mode toggle applies visual masking and obfuscated names instantly.
  - [x] Auto-classification accurately tags new mods containing restricted keywords during scan and writes to `info.json`.
  - [x] PIN verification performed backend-side using `argon2` crate with constant-time comparison.
  - [x] Brute force limited to 5 failed attempts before a 60s memory-backed lockout.
  - [x] App ALWAYS launches in SFW mode (Safe Mode: Enabled) regardless of previous session state.
  - [x] Dual Guard guarantees 0 leakage — NSFW mods are disabled and hidden from counts in SFW mode.

---

# Epic 30: Privacy & Safe Mode

## 1. Executive Summary

- **Problem Statement**: Users manage mods with varying content sensitivity. Opening the app on stream or in public risks displaying NSFW thumbnails and names. A fast, trustworthy privacy layer is required to prevent accidental exposure, while ensuring seamless integration with the user's saved loadouts (Collections).
- **Proposed Solution**: A dual-corridor Safe Mode system (Safe vs Unsafe) backed by the `is_safe` flag on individual mods. It enforces **Dual Guard Isolation**: UI masking (blurring out-of-corridor mods) and **Physical Corridor Handoff** (physically disabling opposing mods using the `DISABLED ` prefix). It integrates tightly with Collections by restoring the `last_active` collection of the target corridor upon switching.
- **Success Criteria**:
  - **Memory & Boot Guard**: The app remembers the last active Safe Mode state. If booting into Unsafe Mode and a PIN is set, the app locks the UI immediately before showing any grid data.
  - **Atomic Corridor Handoff**: Switching corridors physically disables all active mods from the leaving corridor (`disabled_reason = 'SYSTEM'`) and applies the `last_active` collection of the destination corridor.
  - **Crash Resiliency**: Corridor switches are logged in the `tasks` DB table. App crashes during a switch will trigger a `RECOVERY_REQUIRED` dialog on the next boot.
  - **Object Independence**: Top-level Objects are NEVER physically disabled by the Safe Mode switch; only the Mod folders (Depth 1-5) inside them are manipulated.
  - **Auto-Tagging**: New imports containing restricted keywords are automatically tagged `is_safe = false` during the scan engine phase.

---

## 2. User Experience & Functionality

### User Stories

#### US-30.1: Toggle Global Safe Mode (Corridor Handoff)

As a user, I want a quick toggle to switch between my Safe and Unsafe mods, restoring my exact loadout for that specific corridor automatically.

| ID        | Type        | Criteria                                                                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.1.1 | ✅ Positive | Given the Safe Mode shield icon is clicked, the system acquires an `OperationLock` and writes a `PENDING` status to the `tasks` table before touching the filesystem.                                             |
| AC-30.1.2 | ✅ Positive | Given the switch initiates, the backend disables the leaving corridor by prepending `DISABLED ` to all currently ENABLED mods where `is_safe != target_safe_mode`, setting their DB `disabled_reason = 'SYSTEM'`. |
| AC-30.1.3 | ✅ Positive | Given the leaving corridor is disabled, the system restores the target corridor by finding its `last_active == true` collection and invoking `apply_collection`.                                                  |
| AC-30.1.4 | ⚠️ Edge     | Given the target corridor has no saved or unsaved collections, the system falls back to manually enabling mods where `disabled_reason == 'SYSTEM'` and `is_safe == target_safe_mode`.                             |
| AC-30.1.5 | ✅ Positive | Given the handoff completes, the backend updates the `tasks` table to `COMPLETED` and returns `restored_collection_id`, allowing the React Topbar to sync its dropdown immediately.                               |

---

#### US-30.2: Startup Sequence & PIN Security

As a user, I want my privacy to be protected even if I close the app while Unsafe Mode is active, ensuring no one else can open the app and see my NSFW mods.

| ID        | Type        | Criteria                                                                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.2.1 | ✅ Positive | Given the app is launched, the backend reads the last active Safe Mode state from `app_settings`. If the state is Unsafe Mode AND a PIN is configured, the backend emits a `LOCK_UI` event.                                 |
| AC-30.2.2 | ✅ Positive | Given the `LOCK_UI` event, the React frontend renders a full-screen `PinEntryModal` and blurs/hides the main workspace. The grid does not load until `verify_pin()` returns `true`.                                         |
| AC-30.2.3 | ❌ Negative | Given 5 consecutive incorrect PIN attempts, the PIN entry locks for 60 seconds (memory-backed `PinGuardState`).                                                                                                             |
| AC-30.2.4 | ✅ Positive | Given the app boots, if it finds a `status = 'PENDING'` record in the `tasks` table (indicating a crash during a previous mode switch or collection apply), it emits `RECOVERY_REQUIRED` to prompt the user for resolution. |

---

#### US-30.3: UI Masking & Object Independence

As a user, I want my main navigation (Objects) to remain stable regardless of the mode, but sensitive mod thumbnails to be hidden if they leak into the view.

| ID        | Type        | Criteria                                                                                                                                                                                                  |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.3.1 | ✅ Positive | Given Safe Mode is active, the ObjectList displays ALL Objects (Characters/Weapons) normally, but the counts badge ONLY reflects the total number of mods belonging to the Safe Corridor.                 |
| AC-30.3.2 | ✅ Positive | Given an Unsafe mod is somehow displayed while Safe Mode is active (e.g., pending FileWatcher update), its thumbnail is replaced with a blurred placeholder and its name masked via CSS `filter: blur()`. |
| AC-30.3.3 | ❌ Negative | Given a mode switch occurs, the backend NEVER applies the `DISABLED ` prefix to a top-level Object folder, ensuring the ObjectList structure remains intact.                                              |

---

#### US-30.4: Privacy Tagging & Auto-Classification

As a user, I want the system to automatically flag potentially sensitive mods so I don't have to review them manually.

| ID        | Type        | Criteria                                                                                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-30.4.1 | ✅ Positive | Given a folder is scanned, if its name or tokens match `safe_mode_keywords`, the system automatically tags it as `is_safe = false` in the DB.                      |
| AC-30.4.2 | ✅ Positive | Given the context menu, when a user manually toggles "Mark as NSFW", the `is_safe` boolean is updated in the DB immediately.                                       |
| AC-30.4.3 | ⚠️ Edge     | Given a user marks a mod as "Safe" while currently in Unsafe Mode, the mod immediately becomes invisible to the current corridor and will be disabled dynamically. |

---

### Non-Goals

- Safe Mode focuses on UI visibility and disk-level corridor separation (disabling via prefix). It does not encrypt actual mod files (`.dds`, `.buf`).
- No auto-lock timer based on idle activity (only on startup or manual toggle).
- No remote PIN sync or recovery questions. Lost PIN requires manual SQLite database edit.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: PrivacyManager (switch.rs, guard.rs)

pub async fn switch_mode(pool: &SqlitePool, target_safe_mode: bool) -> Result<SwitchResult, Error> {
    // 1. Acquire Locks
    let _lock = acquire_operation_lock().await;

    // 2. Track Task
    let task_id = insert_task(pool, "SWITCH_MODE", target_safe_mode).await?;

    // 3. Disable Leaving Corridor
    // Only target depth 1-5 mods, NEVER top-level objects
    let leaving_mods = get_enabled_mods_where_safe_is_not(pool, target_safe_mode).await?;
    for mod in leaving_mods {
        fs::rename_prepend_disabled(&mod.folder_path).await?;
        update_db_status_and_reason(pool, mod.id, "DISABLED", "SYSTEM").await?;
    }

    // 4. Restore Target Corridor
    let restored_collection_id = if let Some(last_collection) = get_last_active_collection(pool, target_safe_mode).await? {
        // Delegate to Epic 31's logic
        apply_collection_internal(pool, last_collection.id, true).await?;
        Some(last_collection.id)
    } else {
        // Fallback: Enable strictly by SYSTEM reason
        let system_mods = get_system_disabled_mods_for_corridor(pool, target_safe_mode).await?;
        for mod in system_mods {
            fs::rename_remove_disabled(&mod.folder_path).await?;
            update_db_status_and_reason(pool, mod.id, "ENABLED", NULL).await?;
        }
        None
    };

    // 5. Complete Task
    complete_task(pool, task_id).await?;

    Ok(SwitchResult { restored_collection_id })
}

### Integration Points
| Component | Detail |
| --- | --- |
| Boot Guard | React `App.tsx` checks backend payload on mount. Halts render and mounts `PinEntryModal` if `LOCK_UI` is true. |
| Topbar State | `SwitchResult.restored_collection_id` is sent to React, updating Zustand `activeCollectionId` to sync the Dropdown. |
| Task Recovery | Next boot checks `tasks` table. If `status == 'PENDING'`, it halts UI and shows the "Recovery Action" dialog (Resume / Rollback). |
| PIN Hashing | Uses `argon2` crate with constant-time verification. Lockouts stored in memory (`PinGuardState`). |


### Security & Privacy
- **Strict Corridor Enforcement**: Mods with `is_safe != current_mode` cannot be physically enabled. Dual Guard ensures they are both physically renamed and excluded from SQL counts.
- **Fail-Safe Startup**: By recording the corridor switch in the `tasks` table, any power loss during the mass-rename process will be caught and resolved on the next launch, preventing corrupted physical states.

---

## 4. Dependencies
- **Blocked by**: Epic 13 (Core Mod Ops - `rename` logic), Epic 14 (OperationLock), Epic 31 (Collections - `apply_collection` logic).
- **Blocks**: None.
```
