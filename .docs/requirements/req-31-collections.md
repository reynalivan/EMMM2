# Epic 31: Virtual Collections (Loadouts)

## 1. Executive Summary

- **Problem Statement**: Users switch contexts frequently (e.g., streaming, full-overhaul, vanilla), making manual toggling of tens of mods error-prone. Restoring exact loadouts requires a system capable of atomic mass activation, conflict resolution, and self-healing.
- **Proposed Solution**: Virtual Collections built on a `collections` and `collection_items` mapping. The system features a **Full Loadout Swap** (disabling non-collection mods within the same safety corridor), **Double ID Tracking** (ID + path relinking if mod ID changes), `is_safe_context` awareness (Exclusive Corridors), and a **"Last Unsaved" Snapshot** hidden collection for 1-click Undo.
- **Success Criteria**:
  - `apply_collection` for 100 operations (50 enable + 50 disable) completes in ≤ 5s on SSD.
  - **Exclusivity**: Applying a collection automatically disables all currently enabled mods in the same safety context (Safe vs Unsafe corridor) that are not part of the collection.
  - **Persistence**: Collection items successfully relink using their `folder_path` if the mod ID changes (e.g., after a re-scan or DB wipe).
  - **Nested Support**: Deep nested mods (depth 2-3) are correctly captured and toggled via the `nested_walker` filesystem logic.
  - **Undo**: A hidden snapshot collection allows rolling back the entire state in ≤ 5s.
  - **Corridor Isolation**: Collections tagged as Unsafe (`is_safe_context = 0`) are completely hidden from UI and Apply interfaces when Safe Mode is Active.

---

## 2. User Experience & Functionality

### User Stories

#### US-31.1: Loadout Creation & Context Sensitivity

As a user, I want to create mod collections (e.g., "Stream Loadout"), so that I can bundle specific mods into a single click package.

| ID        | Type        | Criteria                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.1.1 | ✅ Positive | Given the Collections tab, when I click "Save Current State", the system captures currently enabled `folder_path` and `mod_id` values into `collection_items`. |
| AC-31.1.2 | ✅ Positive | Given a created Collection, the current Safe Mode state determines its `is_safe_context`. If created while Safe Mode is OFF (contains NSFW), the context is marked as Unsafe (`0`). |
| AC-31.1.3 | ✅ Positive | Given Safe Mode is ON, only Collections with `is_safe_context = 1` are returned by the API. Unsafe collections are entirely hidden to prevent accidental exposure. |
| AC-31.1.4 | ⚠️ Edge     | When a collection is created/updated, its `preset_name` is written to the portable `info.json` file inside each member mod's folder to ensure metadata portability          |

---

#### US-31.2: Exclusive Loadout Swap & Nested Support

As a user, I want to activate a preset and have the app automatically disable all other mods in that corridor, so that I have a clean, predictable environment.

| ID        | Type        | Criteria                                                                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.2.1 | ✅ Positive | Given a collection is Apply-clicked, the system pre-computes the diff. Any mod currently enabled in the same safety context (Safe vs Unsafe) that is NOT in the collection is automatically selected for disable (Exclusive Swap).               |
| AC-31.2.2 | ✅ Positive | Given the diff is calculated, the system engages an `OperationLock` and `WatcherSuppression`, completing the mass physical folder rename (adding/removing `DISABLED ` prefix) cleanly.                                                            |
| AC-31.2.3 | ✅ Positive | Given the collection includes deep nested mods (depth 2-3 inside object folders), the `nested_walker` crawler locates and renames them to match the target state.                                                                                |
| AC-31.2.4 | ❌ Negative | Given the apply fails halfway due to an OS lock, the system halts and logs warnings. The user can use the "Undo" button (if snapshot was created) to revert the partial changes.                                                                   |

---

#### US-31.3: Smart Tracing & Healing (Double ID)

As a user, I want collections to survive folder renames, so that organizing my library doesn't destroy my saved presets.

| ID        | Type        | Criteria                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.3.1 | ✅ Positive | Given a collection item's `mod_id` is missing in the DB during Apply, the system tries to find the mod using its saved `folder_path` (Self-Healing).                                          |
| AC-31.3.2 | ✅ Positive | Given the mod was found via path, the `collection_items` record is updated with the new ID and the Apply continues.                                                                           |
| AC-31.3.3 | ❌ Negative | Given the mod folder no longer exists at the saved path, a warning "Skipping missing mod: {name}" is appended to the result; the rest of the collection is applied.                            |

---

#### US-31.4: Cheat Death (One-Click Undo)

As a user, I want to cancel an applied preset if it looks wrong, so that I can experiment without fear.

| ID        | Type        | Criteria                                                                                                                                                                            |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.4.1 | ✅ Positive | Given a preset applied successfully, a success toast appears with an "Undo" button (active for 10s).                                                |
| AC-31.4.2 | ✅ Positive | Given I click Undo, the system applies the "Last Unsaved" hidden collection (snapshot of enabled state prior to apply), restoring the previous corridor loadout.    |
| AC-31.4.3 | ⚠️ Edge     | Current implementation overwrites the snapshot on every `apply_collection` call, ensuring only the most recent atomic action can be reverted.                      |

---

### Non-Goals

- No multi-level undo history — only the latest apply's snapshot is retained.
- No cross-game collections — presets are strictly bound to a single `game_id`.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: Collections (apply.rs, storage.rs)
// metadata portability via update_info_json triggers

apply_collection(game_id, collection_id):
  1. Acquire OperationLock(game_id)
  2. Snapshot current corridor enabled state → snapshot_current_state()
     - Captures both DB-tracked mods and nested mods (is_enabled && is_safe == context)
  3. Load collection_members of target preset.
  4. Diff: to_disable (active NOT IN target), to_enable (target NOT IN active)
  5. Engage WatcherSuppression via SuppressionGuard.
  6. Atomic Rename: 
     - DB Mods: apply_state_change -> batch_update_mods_status_and_path
       - Sets `disabled_reason = 'COLLECTION'` for newly disabled mods, clears for enabled.
     - Nested Mods: apply_nested_mods renames subfolders directly on disk
  7. Return ApplyCollectionResult { changed_count, warnings }
```

### Integration Points

| Component            | Detail                                                                                                     |
| -------------------- | ---------------------------------------------------------------------------------------------------------- |
| Persistence Tracking | `collection_items` stores `mod_id` + `mod_path`. Relinks via path if ID missing (Double-ID Tracking).       |
| Portable JSON        | `storage::update_info_json` writes preset membership to folder's `info.json` upon save/update/delete.      |
| Exclusive Corridors  | `is_safe_context` separation enforced; `list_collections` and `apply_collection` respect `safe_mode_enabled`.|
| Nested Support       | `nested_walker` crawls ContainerFolders; `apply_nested_mods` renames them to match preset state.           |
| Undo Logic           | `snapshot_current_state` creates/overwrites `is_last_unsaved=1` collection; 10s frontend toast for revert. |
| Suppression          | `SuppressionGuard::new(&watcher_state.suppressor)` prevents watcher storms during bulk renames.            |

### Security & Privacy

- **Leakage Prevention**: API queries strictly isolate results by `safe_mode_enabled`. Unsafe collections never leave the backend if Safe Mode is active.
- **Atomic Operations**: State changes use `OperationLock` and `WatcherSuppression` to prevent race conditions or secondary sync triggers during mass renames.

---

## 4. Dependencies

- **Blocked by**: Epic 20 (Mod Toggle - Standardizer prefix logic), Epic 14 (OperationLock), Epic 30 (Safe Mode state), Epic 09 (Object Schema for Conflict Resolution IDs).
- **Blocks**: Epic 35 (Smart Randomizer - uses collections backend to commit proposals).
