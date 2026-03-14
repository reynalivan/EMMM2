# Epic 31: Virtual Collections (Loadouts)

## 1. Executive Summary

- **Problem Statement**: Users switch contexts frequently (e.g., streaming, full-overhaul, vanilla), making manual toggling of tens of mods error-prone. Restoring exact loadouts requires a system capable of atomic mass activation, conflict resolution, and self-healing.
- **Proposed Solution**: Virtual Collections built on a `collections` and `collection_items` mapping. The system features Smart Conflict Resolution (auto-disabling overlapping objects), Double ID Tracking (path + BLAKE3 hash relinking if renamed), `is_safe_context` awareness, and a Pre-Apply Snapshot for 1-click Undo.
- **Success Criteria**:
  - `apply_collection` for 100 operations (50 enable + 50 disable) completes in ≤ 5s on SSD.
  - Conflict Resolution automatically disables active mods sharing the same `object_id` when a collection mod is enabled.
  - Double ID Tracking successfully relinks a collection item using its BLAKE3 hash if the physical folder path was changed globally.
  - Pre-apply snapshots safely roll back the filesystem in ≤ 5s upon a mid-apply failure.
  - Collections created in NSFW mode are completely hidden from UI and Apply interfaces when Safe Mode is Active.

---

## 2. User Experience & Functionality

### User Stories

#### US-31.1: Loadout Creation & Context Sensitivity

As a user, I want to create mod collections (e.g., "Stream Loadout"), so that I can bundle specific mods into a single click package.

| ID        | Type        | Criteria                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.1.1 | ✅ Positive | Given the Collections tab, when I click "Save Current State", the system captures currently enabled `folder_path` and `folder_hash` (BLAKE3) values into `collection_items` |
| AC-31.1.2 | ✅ Positive | Given a created Collection, the current Safe Mode state determines its `is_safe_context`. If created while Safe Mode is OFF (contains NSFW), the context is marked as NSFW  |
| AC-31.1.3 | ✅ Positive | Given Safe Mode is ON, any Collections with an NSFW `is_safe_context` are entirely hidden from the UI and cannot be applied to prevent accidental leakage                   |
| AC-31.1.4 | ⚠️ Edge     | When a collection is created/updated, its `preset_name` is written to the portable `info.json` file inside each member mod's folder to ensure metadata portability          |

---

#### US-31.2: Atmospheric Mass Swap & Smart Conflict Resolution

As a user, I want to activate a preset and have the app automatically disable any conflicting mods, so that my game doesn't break due to overlapping models.

| ID        | Type        | Criteria                                                                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.2.1 | ✅ Positive | Given a collection is Apply-clicked, the system pre-computes targets. If the preset enables "Raiden Bikini" (`object_id: Raiden`), any currently active mod with the same `object_id` is automatically selected for disable (Conflict Resolution) |
| AC-31.2.2 | ✅ Positive | Given the diff is calculated, the system engages an `OperationLock` and `WatcherSuppression`, completing the mass physical folder rename (adding/removing `DISABLED ` prefix) cleanly                                                             |
| AC-31.2.3 | ✅ Positive | Given the collection has deep nested mods, the system uses a dynamic recursive crawler (`nested_walker`) to locate and rename (`DISABLED `) specific sub-level variants precisely to match the snapshot state                                     |
| AC-31.2.4 | ❌ Negative | Given the apply fails halfway due to an OS lock, the system immediately halts, reads the pre-apply snapshot, reverses changes, and toasts "Apply failed — rolled back"                                                                            |

---

#### US-31.3: Smart Tracing & Healing (Double ID)

As a user, I want collections to survive folder renames, so that organizing my library doesn't destroy my saved presets.

| ID        | Type        | Criteria                                                                                                                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.3.1 | ✅ Positive | Given a collection item's `folder_path` is missing on disk during Apply, the system queries the DB by the item's saved BLAKE3 hash to find the new path (Self-Healing)                        |
| AC-31.3.2 | ✅ Positive | Given the folder was found via hash, the `collection_items` record is updated with the new path and the Apply continues                                                                       |
| AC-31.3.3 | ❌ Negative | Given the missing item's hash is also nowhere to be found (mod was permanently deleted), a warning "Skipping missing mod: {name}" is appended to the success toast; the rest process normally |

---

#### US-31.4: Cheat Death (One-Click Undo)

As a user, I want to cancel an applied preset if it looks wrong, so that I can experiment without fear.

| ID        | Type        | Criteria                                                                                                                                                                            |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.4.1 | ✅ Positive | Given a preset applied successfully, a success toast appears with an "Undo" button (active for 10s)                                                                                 |
| AC-31.4.2 | ✅ Positive | Given I click Undo, `undo_latest_apply` reads the snapshot JSON (the enabled mods prior to the apply action) and restores the exact previous Enable/Disable folder prefixes in ≤ 5s |
| AC-31.4.3 | ⚠️ Edge     | Given I apply a second collection consecutively before clicking the first Undo, the snapshot is overwritten by the latest state, ensuring rolling forward cleanly is the priority   |

---

### Non-Goals

- No multi-level undo history — only the latest apply's snapshot is retained.
- No cross-game collections — presets are strictly bound to a single `game_id`.

---

## 3. Technical Specifications

### Architecture Overview

```rust
DB Schema:
  collections (id, game_id, name, is_safe_context, created_at)
  collection_items (id, collection_id, folder_path, folder_hash) // Double ID
  apply_snapshots (id, collection_id, game_id, snapshot_json, applied_at)

apply_collection(game_id, collection_id):
  1. Acquire OperationLock(game_id)
  2. Snapshot current enabled state → INSERT INTO apply_snapshots
  3. Load collection_items. Validate paths; if missing, lookup by folder_hash.
  4. Fetch enabled mods. Compute Conflict Set (active mods sharing an object_id with a preset mod).
  5. Compute diff:
        to_disable = (currently_enabled NOT IN preset) OR IN Conflict Set
        to_enable = preset NOT IN currently_enabled
  6. Acquire WatcherSuppression(all_paths)
  7. fs::rename to add/remove "DISABLED "
     → on error: rollback via snapshot
  8. Return ApplyResult { enabled, disabled, warnings }
```

### Integration Points

| Component          | Detail                                                                            |
| ------------------ | --------------------------------------------------------------------------------- |
| Double ID Tracking | Storing `folder_path` + `folder_hash` to heal broken preset links dynamically.    |
| Portable JSON      | `preset_name` synced to `info.json` for external persistence.                     |
| Context Logic      | `is_safe_context` queries tie into Epic 30's Safe Mode Zustand state.             |
| Conflict Resolver  | SQL filter matching `object_id` against active mods during preset initialization. |

### Security & Privacy

- **Leakage Prevention**: System completely omits NSFW collections from API returns when Safe Mode is active via mandatory `WHERE is_safe_context = false` injection.
- **Rollback Guarantee**: Pre-apply snapshot commits purely to SQLite DB before physical execution starts, ensuring the map home is safe if a power loss occurs mid-rename.

---

## 4. Dependencies

- **Blocked by**: Epic 20 (Mod Toggle - Standardizer prefix logic), Epic 14 (OperationLock), Epic 30 (Safe Mode state), Epic 09 (Object Schema for Conflict Resolution IDs).
- **Blocks**: Epic 35 (Smart Randomizer - uses collections backend to commit proposals).
