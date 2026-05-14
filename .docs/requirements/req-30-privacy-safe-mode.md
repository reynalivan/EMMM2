# Epic 30: Privacy & Safe Mode

## 1. Executive Summary

- **Problem Statement**: Users manage mods with varying content sensitivity. Opening the app on stream or in public risks displaying NSFW thumbnails and names. A fast, trustworthy privacy layer is required to prevent accidental exposure, while ensuring seamless integration with the user's saved loadouts (Collections).
- **Proposed Solution**: A dual-corridor Safe Mode system (Safe vs Unsafe) backed by the `is_safe` flag on individual mods. It enforces **Dual Guard Isolation**: UI masking (blurring out-of-corridor mods) and **Physical Corridor Handoff** (physically disabling opposing mods using the `DISABLED ` prefix). It integrates tightly with Collections by restoring the target corridor from its own resolved state in priority order: `active_collection_id` if valid, otherwise the corridor-scoped Unsaved collection, otherwise SYSTEM fallback.
- **Success Criteria**:
  - **Backend-Authoritative Corridor**: The backend `safe_mode.enabled` setting is the active corridor source of truth. React syncs from command results instead of assuming the requested target is active.
  - **Boot Guard**: The app remembers the last active Safe Mode state. If booting into Unsafe Mode and a PIN is set, the app locks the UI immediately before showing any grid data.
  - **Atomic Corridor Handoff**: Switching corridors physically disables all active mods from the leaving corridor (`disabled_reason = 'SYSTEM'`) and restores the destination corridor from its own `active_collection_id`, corridor-scoped Unsaved collection, or SYSTEM fallback through the shared runtime mutation engine.
  - **Crash Resiliency**: Corridor switches are logged in the `tasks` DB table. App crashes during a switch will trigger a `RECOVERY_REQUIRED` dialog on the next boot.
  - **Object Independence**: Top-level Objects are NEVER physically disabled by the Safe Mode switch; only the Mod folders (Depth 1-5) inside them are manipulated.
  - **Auto-Tagging**: New imports containing restricted keywords are automatically tagged `is_safe = false` during the scan engine phase.

---

## 2. User Experience & Functionality

### User Stories

#### US-30.1: Toggle Global Safe Mode (Corridor Handoff)

As a user, I want a quick toggle to switch between my Safe and Unsafe mods, restoring my exact loadout for that specific corridor automatically.

| ID        | Type        | Criteria                                                                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.1.1 | ✅ Positive | Given the Safe Mode shield icon is clicked, the system acquires an `OperationLock` and writes a `PENDING` status to the `tasks` table before touching the filesystem.                                                       |
| AC-30.1.2 | ✅ Positive | Given the switch initiates, the backend disables the leaving corridor by prepending `DISABLED ` to all currently ENABLED mods where `is_safe != target_safe_mode`, setting their DB `disabled_reason = 'SYSTEM'`.           |
| AC-30.1.3 | ✅ Positive | Given the leaving corridor is disabled, the system restores the target corridor by resolving its own target state in priority order: valid `active_collection_id` -> corridor-scoped Unsaved collection -> SYSTEM fallback. |
| AC-30.1.4 | ⚠️ Edge     | Given the target corridor has no valid active or unsaved collection, the system falls back to manually enabling mods where `disabled_reason == 'SYSTEM'` and `is_safe == target_safe_mode`.                                 |
| AC-30.1.5 | ✅ Positive | Given the handoff completes, the backend persists `safe_mode.enabled`, updates the `tasks` table to `COMPLETED`, and returns `active_safe`, `restored_collection_id`, and `warnings` so React syncs from backend state.     |
| AC-30.1.6 | ✅ Positive | Given both corridors are currently unsaved, the switch preview dialog and Topbar surfaces use the same canonical labels: `Unsaved SAFE Preset` for Safe and `Unsaved UNSAFE Preset` for Unsafe.                             |

---

#### US-30.2: Startup Sequence & PIN Security

As a user, I want my privacy to be protected even if I close the app while Unsafe Mode is active, ensuring no one else can open the app and see my NSFW mods.

| ID        | Type        | Criteria                                                                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.2.1 | ✅ Positive | Given the app is launched, the backend reads the last active Safe Mode state from `app_settings`. If the state is Unsafe Mode AND a PIN is configured, the backend emits a `LOCK_UI` event.                                 |
| AC-30.2.2 | ✅ Positive | Given the `LOCK_UI` event, the React frontend renders a full-screen `PinEntryModal` and blurs/hides the main workspace. The grid does not load until `verify_pin()` returns `true`.                                         |
| AC-30.2.3 | ❌ Negative | Given 5 consecutive incorrect PIN attempts, the PIN entry locks for 60 seconds using DB-backed `pin_config.failed_attempts` and `pin_config.lockout_until`, so restart does not reset the lockout.                          |
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
// Backend entrypoint: collections::cmds::switch_corridor
// Runtime owner: pipeline::switch_pipeline + services::runtime_mutation_engine

pub async fn switch_corridor(game_id: String, target_safe_mode: bool) -> Result<CorridorSwitchResult, Error> {
    // 1. Resolve active settings/corridor pointers and acquire OperationLock.
    let watcher_state = app.state::<WatcherState>();
    let mut ctx = switch_pipeline::SwitchContext::new(pool, game_id, target_safe_mode).await?;

    // 2. Execute the shared corridor switch pipeline.
    // The pipeline records task state, disables the leaving corridor, restores the
    // target corridor from active/unsaved collection or SYSTEM fallback, and calls
    // runtime_mutation_engine::toggle_mods_mixed under WatcherSuppression.
    let result = switch_pipeline::execute(&mut ctx, watcher_state).await?;

    // 3. Return backend-authoritative corridor state for React to sync from.
    Ok(result)
}
```

### Integration Points

| Component         | Detail                                                                                                                                                                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Boot Guard        | React `App.tsx` checks backend payload on mount. Halts render and mounts `PinEntryModal` if `LOCK_UI` is true.                                                                                                                             |
| Topbar State      | `SwitchResult.restored_collection_id` is sent to React, updating Zustand `activeCollectionId` to sync the Dropdown. Unsaved corridor display names shown after the switch must come from the same shared label source used by Collections. |
| Workspace Runtime | `WorkspaceViewModel.explorer` is corridor-filtered backend-side. `ObjectList` stays all-objects, while `Preview` must drop stale `selected_mod_path` values that no longer belong to the active corridor.                                  |
| Task Recovery     | Next boot checks `tasks` table. If `status == 'PENDING'`, it halts UI and shows the "Recovery Action" dialog (Resume / Rollback).                                                                                                          |
| PIN Hashing       | Uses `argon2` crate with constant-time verification. Failed attempts and lockout expiry are persisted in `pin_config`.                                                                                                                     |

### Security & Privacy

- **Strict Corridor Enforcement**: Mods with `is_safe != current_mode` cannot be physically enabled. Dual Guard ensures they are both physically renamed and excluded from SQL counts.
- **Fail-Safe Startup**: By recording the corridor switch in the `tasks` table, any power loss during the mass-rename process will be caught and resolved on the next launch, preventing corrupted physical states.

---

## 4. Dependencies

- **Blocked by**: Epic 13 (Core Mod Ops - `rename` logic), Epic 14 (OperationLock), Epic 31 (Collections - `apply_collection` logic).
- **Blocks**: None.
