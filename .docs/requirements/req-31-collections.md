# Epic 31: Virtual Collections (Loadouts)

## 1. Executive Summary

- **Problem Statement**: Users frequently switch gameplay contexts (e.g., streaming, full-overhaul, vanilla), making manual toggling of dozens of mods error-prone. Restoring exact loadouts requires a system capable of atomic mass activation, conflict resolution, graceful failure handling, and self-healing when folders are moved.
- **Proposed Solution**: Virtual Collections built on a robust `collections`, `collection_mods`, and `collection_objects` mapping. The system features an **Exclusive Swap** (disabling non-collection mods within the same safety corridor), **Dirty State Tracking** (auto-generating an `Unsaved` collection on manual/FileWatcher edits), **Pre-Apply Disk Validation** (preventing ghost applies), **Cross-Collection Auto-Healing** (updating paths if mods move), and **Task Recovery** to resume operations interrupted by app crashes.
- **Success Criteria**:
  - `apply_collection` exclusively enables target mods and disables the rest within the same safety corridor in ≤ 5s for 100 mods on an SSD.
  - Manual mod toggles or external FileWatcher events immediately flag the state as "Dirty", generating an `Unsaved Collection` (format: `YYYYMMDDXXXX`) as the active state.
  - Pre-Apply Validation accurately detects physically missing mods and prompts a resolution dialog (Skip/Cancel) before any disk mutations occur.
  - Moving or renaming a mod cascades path updates to all saved collections automatically.
  - App crashes during `apply_collection` or Safe Mode `switch_mode` are caught on next boot via the `tasks` table, prompting a Recovery Action.

---

## 2. User Experience & Functionality

### User Stories

#### US-31.1: Context-Sensitive Creation & Save As

As a user, I want to save my currently active mods as a permanent collection, so that I can easily revert to this exact setup later.

| ID        | Type        | Criteria                                                                                                                                                                                                                     |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.1.1 | ✅ Positive | Given the active state is an Unsaved Collection, when I click "Save Collection", the system prompts for a name and performs a "Save As" operation — creating a new permanent collection and deleting the old Unsaved record. |
| AC-31.1.2 | ✅ Positive | Given the Save operation is triggered, the backend validates all active mods against the physical disk; if a mod has 0 active items, the save is rejected with a "Cannot save an empty collection" error.                    |
| AC-31.1.3 | ✅ Positive | Given a created Collection, the current Safe Mode state determines its `is_safe_context`. If created in Safe Mode, it is hidden entirely when the app is in Unsafe Mode, ensuring zero cross-corridor leakage.               |
| AC-31.1.4 | ❌ Negative | Given I click Save but a physical folder for an active mod has just been deleted externally by the user, the disk validation fails to find it and silently drops it from the saved payload — the Disk is the Absolute Truth. |

---

#### US-31.2: Pre-Apply Validation & Exclusive Swap

As a user, I want to activate a preset and have the app automatically disable all other mods, while warning me if any saved mods have gone missing from my disk.

| ID        | Type        | Criteria                                                                                                                                                                                                                                         |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-31.2.1 | ✅ Positive | Given a collection is clicked, the system runs a Pre-Apply Validation. If all physical mod paths exist, the system proceeds to the Exclusive Swap automatically.                                                                                 |
| AC-31.2.2 | ❌ Negative | Given some mods in the collection are physically missing from the disk, the backend returns a `MissingModsError` array. The React UI intercepts this and displays a "Missing Mods" dialog listing the lost paths.                                |
| AC-31.2.3 | ✅ Positive | Given the Missing Mods dialog, if the user clicks "Skip & Apply", the frontend re-triggers the apply command with `ignore_missing = true`, skipping the lost mods and proceeding with the swap.                                                  |
| AC-31.2.4 | ✅ Positive | Given the swap executes, it acquires an `OperationLock`, suppresses the Watcher, removes `DISABLED ` from targets, and prepends `DISABLED ` to non-targets in the same corridor, setting `disabled_reason = 'COLLECTION'` for the disabled ones. |
| AC-31.2.5 | ⚠️ Edge     | Given the collection contains multiple active mods for the same Object (e.g., two skins for Albedo), the automated apply ignores/bypasses standard duplicate hash warnings and applies them simultaneously.                                      |

---

#### US-31.3: Dirty State & Topbar Synchronization

As a user, I want the system to recognize when I manually modify an active preset, so that my changes are tracked and my Topbar reflects the "Unsaved" status.

| ID        | Type        | Criteria                                                                                                                                                                                                               |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.3.1 | ✅ Positive | Given a saved Collection is active, when I toggle a mod manually OR a FileWatcher event detects a folder rename, the state becomes "Dirty".                                                                            |
| AC-31.3.2 | ✅ Positive | Given a Dirty event, the backend automatically upserts ONE `is_unsaved = true` collection for the current corridor, named with the timestamp (e.g., `202603230850`), and snapshots the currently enabled mods into it. |
| AC-31.3.3 | ✅ Positive | Given the Unsaved Collection is created, the React Topbar immediately updates its selection to this new collection and renders an "Unsaved \*" badge next to the name.                                                 |
| AC-31.3.4 | ⚠️ Edge     | Given the user manually switches back to the original saved Collection via the Topbar, the system applies it, overriding the Unsaved changes, and updates the Topbar to the clean state.                               |

---

#### US-31.4: Cross-Collection Auto-Healing

As a user, I want my saved collections to remain intact even if I move a mod folder to a different Object category or rename it.

| ID        | Type        | Criteria                                                                                                                                                                                                    |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.4.1 | ✅ Positive | Given a user moves a mod via the "Move to Object" dialog, the backend triggers `handle_mod_moved_or_renamed`.                                                                                               |
| AC-31.4.2 | ✅ Positive | Given `handle_mod_moved_or_renamed` fires, the system updates `mod_path` and `object_id` for that specific `mod_id` across ALL saved collections in the database.                                           |
| AC-31.4.3 | ✅ Positive | Given the path cascades successfully, the user can apply a collection from 3 months ago and it will correctly activate the mod in its newly moved location without triggering a Pre-Apply Validation error. |

---

#### US-31.5: Task Recovery (Crash Resiliency)

As a system, I want to track mass I/O operations, so that if the app is killed forcefully, it can recover safely on the next boot.

| ID        | Type        | Criteria                                                                                                                                                                                                                 |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-31.5.1 | ✅ Positive | Given `apply_collection` or `switch_mode` starts, it writes a `PENDING` record to the `tasks` DB table containing the operation payload.                                                                                 |
| AC-31.5.2 | ✅ Positive | Given the operation finishes successfully, the `tasks` record is marked `COMPLETED`.                                                                                                                                     |
| AC-31.5.3 | ❌ Negative | Given the app is force-closed during the rename loop, the `tasks` record remains `PENDING`. On next boot, the backend emits `RECOVERY_REQUIRED` to the frontend.                                                         |
| AC-31.5.4 | ✅ Positive | Given the `RECOVERY_REQUIRED` event, the UI blocks the grid and shows a Dialog: "An operation was interrupted. Resume or Abort?", allowing the backend to re-run the Pre-Apply Validation and finish the remaining loop. |

---

#### US-31.6: Active Collection Deletion

As a user, I want to delete a preset I no longer need without it altering the physical mods that are currently active in my game.

| ID        | Type        | Criteria                                                                                                                                                                                                            |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.6.1 | ✅ Positive | Given the user deletes a Collection that is currently NOT active, the DB record is deleted and nothing else changes.                                                                                                |
| AC-31.6.2 | ✅ Positive | Given the user deletes the currently active Collection, the physical disk is NOT altered (no mods are disabled).                                                                                                    |
| AC-31.6.3 | ✅ Positive | Given the active collection is deleted, the backend immediately snapshots the current active disk state into a new `Unsaved Collection` (`YYYYMMDDXXXX`) and sets it as active in the Topbar to prevent state loss. |

---

### Non-Goals

- No multi-level Undo history (Undo is implicitly handled by re-selecting a previous collection).
- No cross-game collections (collections are strictly scoped to `game_id`).
- No writing collection names into the portable `info.json` (collections are purely DB-driven for faster I/O).

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: Collections (apply.rs, storage.rs, recovery.rs)

apply_collection(game_id, collection_id, ignore_missing):
  1. Insert into `tasks` table -> status = 'PENDING'.
  2. Pre-Apply Validation:
     - Query collection_mods.
     - Check `Path::exists()` for all items.
     - If missing && !ignore_missing -> Return MissingModsError(paths).
  3. Acquire OperationLock(game_id) & Suppress Watcher.
  4. Exclusive Diff Calculation:
     - `to_enable`: Targets that are currently disabled.
     - `to_disable`: Non-targets currently enabled in the same `is_safe_context`.
  5. Atomic FS Rename Loop:
     - Apply/strip "DISABLED " up to depth 5 for `ModPackRoot`/`VariantContainer`.
     - Cascade DB updates (`status`, `folder_path`).
     - Set `disabled_reason = 'COLLECTION'` for disabled, `NULL` for enabled.
  6. DB State Update: Set target collection `last_active = true`, others false.
  7. Update `tasks` table -> status = 'COMPLETED'.
  8. Return `ApplyCollectionResult { collection_id, changed_count }`.

handle_dirty_state(game_id, current_context):
  1. Triggered by IPC from React or Watcher event.
  2. Upsert `is_unsaved = true` collection for `current_context`.
  3. Rename to `chrono::Local::now().format("%Y%m%d%H%M")`.
  4. Snapshot current ENABLED mods/objects into DB.
  5. Set `last_active = true`.

### Integration Points

| Component | Detail |
| --- | --- |
| Topbar State Sync | Commands return `active_collection_id`; React binds Topbar Dropdown value directly to `activeCollectionId` Zustand state. |
| ObjectList Payload | Backend queries MUST return `active_mod_paths: string[]` per Object so the UI Preview Panel correctly highlights active mods. |
| FileWatcher Hook | `notify` crate events map to `handle_dirty_state` to immediately mark manual Explorer changes as Unsaved. |
| Move/Rename Hook | `move_mod` command directly calls `UPDATE collection_mods SET mod_path...` to ensure Auto-Healing. |

### Security & Privacy
- **Corridor Enforcement**: A Safe Mode collection query strictly appends `AND is_safe_context = ?`. An unsafe collection is mathematically impossible to apply while the frontend is in Safe Mode.
- **SSoT Validation**: The `save_collection` command strictly validates `Path::exists()` before committing to DB, preventing ghost entries from creeping into long-term storage.

---

## 4. Dependencies
- **Blocked by**: Epic 13 (Core Mod Ops - `rename_mod`), Epic 14 (OperationLock), Epic 28 (File Watcher), Epic 30 (Safe Mode - `switch_mode` uses these APIs).
- **Blocks**: Epic 35 (Smart Randomizer - uses collections backend to commit proposals).
```
