# Epic 31: Virtual Collections (Loadouts)

## 1. Executive Summary

- **Problem Statement**: Users frequently switch gameplay contexts (e.g., streaming, full-overhaul, vanilla), making manual toggling of dozens of mods error-prone. Restoring exact loadouts requires a system capable of atomic mass activation, conflict resolution, graceful failure handling, and self-healing when folders are moved.
- **Proposed Solution**: Virtual Collections built on a robust `collections`, `collection_mods`, and `collection_objects` mapping. The system features an **Exclusive Swap** (disabling non-collection mods within the same safety corridor), **Dirty State Tracking** (auto-generating an `Unsaved` collection on manual or Disk Reconcile runtime edits), **Pre-Apply Disk Validation** (preventing ghost applies), **Cross-Collection Auto-Healing** (updating paths if mods move), **preview-tree metadata persistence** (`preview_path`, `node_type`, `warnings_json`), and **Task Recovery** to resume operations interrupted by app crashes. Filesystem toggles for collection apply, safe/unsafe switch, and manual runtime toggle share the same runtime mutation engine.
- **Success Criteria**:
  - `apply_collection` exclusively enables target mods and disables the rest within the same safety corridor in ≤ 5s for 100 mods on an SSD.
  - Manual mod toggles or external Disk Reconcile results immediately flag the state as "Dirty", generating a single corridor-scoped Unsaved collection as the active state.
  - Pre-Apply Validation accurately detects physically missing mods before any disk mutations occur. `ignore_missing = false` returns `MissingModsError`; `ignore_missing = true` skips missing members and returns warnings.
  - Moving or renaming a mod cascades path updates to all saved collections automatically.
  - App crashes during `apply_collection` or Safe Mode `switch_mode` are caught on next boot via the `tasks` table, prompting a Recovery Action.

---

## 2. User Experience & Functionality

### User Stories

#### US-31.1: Context-Sensitive Creation & Save As

As a user, I want to save my currently active mods as a permanent collection, so that I can easily revert to this exact setup later.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                  |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.1.1 | ✅ Positive | Given the active state is an Unsaved Collection, when I click "Save Collection", the system prompts for a name and performs a "Save As" operation — creating a new permanent collection and deleting the old Unsaved record.                                              |
| AC-31.1.2 | ✅ Positive | Given the Save operation is triggered, the backend validates all active mods against the physical disk; if a mod has 0 active items, the save is rejected with a "Cannot save an empty collection" error.                                                                 |
| AC-31.1.3 | ✅ Positive | Given a created Collection, the current Safe Mode state determines its `is_safe_context`. If created in Safe Mode, it is hidden entirely when the app is in Unsafe Mode, ensuring zero cross-corridor leakage.                                                            |
| AC-31.1.4 | ❌ Negative | Given I click Save but a physical folder for an active mod has just been deleted externally by the user, the saved collection keeps a logical missing reference and marks it as missing in preview/apply state until the user explicitly updates the original collection. |

---

#### US-31.2: Pre-Apply Validation & Exclusive Swap

As a user, I want to activate a preset and have the app automatically disable all other mods, while warning me if any saved mods have gone missing from my disk.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.2.1 | ✅ Positive | Given a collection is clicked, the system runs a Pre-Apply Validation. If all physical mod paths exist, the system proceeds to the Exclusive Swap automatically.                                                                                                                                |
| AC-31.2.2 | ❌ Negative | Given some mods in the collection are physically missing from the disk and `ignore_missing = false`, the backend returns a `MissingModsError` array before any rename. The React UI intercepts this and displays a "Missing Mods" dialog listing the lost paths.                                |
| AC-31.2.3 | ✅ Positive | Given the Missing Mods dialog, if the user clicks "Skip & Apply", the frontend re-triggers the apply command with `ignore_missing = true`, skipping the lost mods, returning skip warnings, and proceeding with the swap.                                                                       |
| AC-31.2.4 | ✅ Positive | Given the swap executes, it acquires an `OperationLock`, suppresses the Watcher, and delegates all enabled/disabled filesystem rename plus DB projection updates to the shared runtime mutation engine, setting `disabled_reason = 'COLLECTION'` for disabled mods and `NULL` for enabled mods. |
| AC-31.2.5 | ⚠️ Edge     | Given the collection contains multiple active mods for the same Object (e.g., two skins for Albedo), the automated apply ignores/bypasses standard duplicate hash warnings and applies them simultaneously.                                                                                     |

---

#### US-31.3: Dirty State & Topbar Synchronization

As a user, I want the system to recognize when I manually modify an active preset, so that my changes are tracked and my Topbar reflects the "Unsaved" status.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.3.1 | ✅ Positive | Given a saved Collection is active, when I toggle a mod manually OR Disk Reconcile detects an external rename/move/INI/info change, the state becomes "Dirty".                                                                                                          |
| AC-31.3.2 | ✅ Positive | Given a Dirty event, the backend automatically upserts ONE `is_unsaved = true` collection for the current corridor, snapshots the currently enabled mods into it, and keeps any internal timestamp/raw DB name implementation detail hidden from the user-facing label. |
| AC-31.3.3 | ✅ Positive | Given the Unsaved Collection is created, Collection List, Topbar trigger, Topbar dropdown/context panel, Collection Preview, and switcher dialog all render the same canonical display name for that corridor-scoped unsaved state.                                     |
| AC-31.3.4 | ⚠️ Edge     | Given the user manually switches back to the original saved Collection via the Topbar, the system applies it, overriding the Unsaved changes, and updates the Topbar to the clean state.                                                                                |
| AC-31.3.5 | ✅ Positive | Given the active collection is shown in the Topbar dropdown, clicking that same active collection does NOT open an Apply dialog and does NOT trigger a self-apply operation.                                                                                            |
| AC-31.3.6 | ✅ Positive | Given the active corridor state is unsaved, the canonical user-facing labels are corridor-aware: `Unsaved SAFE Preset` for the Safe corridor and `Unsaved UNSAFE Preset` for the Unsafe corridor.                                                                       |

---

#### US-31.4: Cross-Collection Auto-Healing

As a user, I want my saved collections to remain intact even if I move a mod folder to a different Object category or rename it.

| ID        | Type        | Criteria                                                                                                                                                                                                    |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.4.1 | ✅ Positive | Given a user moves a mod via the "Move to Object" dialog, the backend collection auto-healing service runs inside the mutation boundary.                                                                    |
| AC-31.4.2 | ✅ Positive | Given auto-healing runs, the system updates `mod_path` and `object_id` for that specific `mod_id` across ALL saved collections in the database.                                                             |
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

| ID        | Type        | Criteria                                                                                                                                                                                                                                     |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.6.1 | ✅ Positive | Given the user deletes a Collection that is currently NOT active, the DB record is deleted and nothing else changes.                                                                                                                         |
| AC-31.6.2 | ✅ Positive | Given the user deletes the currently active Collection, the physical disk is NOT altered (no mods are disabled).                                                                                                                             |
| AC-31.6.3 | ✅ Positive | Given the active collection is deleted, the backend immediately snapshots the current active disk state into a new corridor-scoped Unsaved collection and sets it as active in the same request, avoiding any transient `NULL` active state. |

---

#### US-31.7: Preview TreeView Semantics & Counting

As a user, I want collection previews to reflect the real active hierarchy of my folders, so that I can understand which active mods will resolve, which branches are blocked by disabled parents, and how many active mods the preset effectively contains.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-31.7.1 | ✅ Positive | Given a collection preview is rendered, Object folders appear as the root level in the treeview, and any relevant `ContainerFolder` ancestors are rendered in full depth so the user can understand the hierarchy.                                                                                                                      |
| AC-31.7.2 | ✅ Positive | Given an active snapshot path resolves to a `VariantContainer`, the preview renders only the main `VariantContainer` row and stops there; child variant folders or leaf mods under that container are not shown in the tree.                                                                                                            |
| AC-31.7.3 | ✅ Positive | Given an active snapshot path resolves to a `ModPackRoot`, the preview renders only the main mod-pack folder row; internal asset subfolders are not shown in the tree.                                                                                                                                                                  |
| AC-31.7.4 | ✅ Positive | Given an active snapshot path resolves to a `FlatModRoot`, the preview renders a terminal leaf using the flat mod file/folder name only, without expanding artificial subfolder noise below the Object root.                                                                                                                            |
| AC-31.7.5 | ✅ Positive | Given an enabled child mod exists under a disabled ancestor `ContainerFolder`, that branch is moved into a dedicated bottom section (for non-impactful/inactive-container branches), the first disabling parent shows a disabled-state chip + warning tooltip, and enabled descendants show `Disabled by Container`.                    |
| AC-31.7.6 | ✅ Positive | Given a terminal preview node is corrupt (for example a `VariantContainer` with a 0 KB root `.ini`), the preview renders a single terminal node only once, preserves its terminal type chip, and shows a warning icon/tooltip instead of duplicating subfolders or sibling rows.                                                        |
| AC-31.7.7 | ✅ Positive | Given a collection contains nested `ContainerFolder` chains, `VariantContainer` roots, and `ModPackRoot` roots, the displayed mod count uses preview-tree semantics: terminal `Mod` leaves count as 1, visible `VariantContainer` rows count as 1, visible `ModPackRoot` rows count as 1, and parent `ContainerFolder` rows count as 0. |
| AC-31.7.8 | ❌ Negative | Given the raw snapshot stores multiple child rows under one `VariantContainer` or `ModPackRoot`, the list/grid/topbar must NOT display the raw snapshot row count; all collection counts must be derived from the collapsed preview tree representation.                                                                                |
| AC-31.7.9 | ✅ Positive | Given collections are saved to disk, `collection_mods` persists enough preview metadata (`preview_path`, `node_type`, `warnings_json`) so variant/mod-pack/flat preview semantics can be reconstructed without relying solely on live path reclassification.                                                                            |

---

### Non-Goals

- No multi-level Undo history (Undo is implicitly handled by re-selecting a previous collection).
- The legacy collection undo IPC is removed; `undo_collection_id` remains only as corridor state metadata for recovery and preview semantics.
- No cross-game collections (collections are strictly scoped to `game_id`).
- No writing collection names into the portable `info.json` (collections are purely DB-driven for faster I/O).
- Collection apply and corridor switch must keep `WorkspaceViewModel.explorer` and Preview aligned to the active corridor; applying a collection must never surface mods from the opposite corridor in the main runtime grid/panel.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: Collections (apply.rs, storage.rs, recovery.rs, preview.rs)

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
  5. Runtime Mutation Engine:
     - Resolve targets by `mod_id` or canonical `folder_path_key`.
     - Preflight missing paths, path traversal, target collisions, and no-op plans before renaming.
     - Apply/strip "DISABLED " for target mod folders.
     - Cascade DB projection updates (`status`, `folder_path`, `folder_path_key`, `disabled_reason`) in the same mutation boundary.
     - Roll back successful filesystem renames best-effort if DB projection update fails.
  6. DB State Update: Set target collection `last_active = true`, others false.
  7. Update `tasks` table -> status = 'COMPLETED'.
  8. Return `ApplyCollectionResult { collection_id, changed_count, warnings }`.

dirty-state snapshot service:
  1. Triggered by Disk Reconcile runtime results or explicit backend runtime mutations.
  2. Upsert `is_unsaved = true` collection for `current_context`.
  3. Internal name may use a timestamp, but all user-facing surfaces resolve the same canonical corridor-aware unsaved label.
  4. Snapshot current ENABLED mods/objects into DB, including collection preview metadata per mod (`preview_path`, `node_type`, `warnings_json`).
  5. Set the unsaved collection as active for that same corridor immediately.

build_collection_preview_tree(objects, mods, mods_path):
  1. Group snapshot rows by Object and use `CollectionObject.path_key` as the folder anchor.
  2. Resolve each member to a terminal preview target using stored metadata first (`preview_path`, `node_type`, `warnings_json`), then fall back to Epic 11 classification only when metadata is absent.
  3. Preserve full `ContainerFolder` chains in the rendered tree.
  4. Collapse terminal display rules:
     - `FlatModRoot` -> terminal leaf
     - `ModPackRoot` -> terminal folder row only
     - `VariantContainer` -> terminal folder row only
  5. If any ancestor `ContainerFolder` is disabled, route that branch into a dedicated bottom section for inactive/non-impactful container branches.
  6. Attach warning tooltip metadata (for example corrupt 0 KB `.ini`) directly to the terminal preview node.
  7. Derive display counts from the preview tree:
     - visible `Mod` leaf = 1
     - visible `VariantContainer` = 1
     - visible `ModPackRoot` = 1
     - inactive-container section rows = 0
     - `ContainerFolder` / Object rows = 0
```

### Integration Points

| Component               | Detail                                                                                                                                                                                                                                                              |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Topbar State Sync       | Commands return `active_collection_id`; React binds Topbar Dropdown value directly to `activeCollectionId` Zustand state. Active collection rows in the Topbar dropdown are non-applicable and must not open self-apply dialogs.                                    |
| Collection Display Name | Collection List, Topbar trigger, Topbar dropdown/context panel, Collection Preview, and switcher dialog must all use the same shared unsaved display-name resolver. User-facing unsaved names are corridor-aware (`Unsaved SAFE Preset` / `Unsaved UNSAFE Preset`). |
| Collection List Count   | `CollectionSummary.mod_count` MUST be derived from preview-tree semantics, not raw `collection_mods` row count, so list numbers match the preview hierarchy.                                                                                                        |
| Preview Panel Payload   | Collection preview, apply preview, and corridor switch preview MUST expose ready-to-render tree payloads rather than requiring the frontend to infer hierarchy from flat member rows.                                                                               |
| Snapshot Persistence    | `collection_mods` persists `preview_path`, `node_type`, and `warnings_json` so saved collection previews remain stable for flat/mod-pack/variant semantics and warning badges.                                                                                      |
| ObjectList Payload      | Backend queries MUST return `active_mod_paths: string[]` per Object so the UI Preview Panel correctly highlights active mods.                                                                                                                                       |
| Disk Reconcile Hook     | Disk Reconcile results map to the collection dirty-state snapshot service to immediately mark manual Explorer changes as Unsaved.                                                                                                                                   |
| Move/Rename Hook        | Backend move/rename mutations update `collection_mods.mod_path` through the collection auto-healing service.                                                                                                                                                        |

### Runtime Dirty-State Trigger Matrix

| Trigger                                                                 | Owner                               | Marks Collection Dirty         | Emits `disk_reconcile:result` |
| ----------------------------------------------------------------------- | ----------------------------------- | ------------------------------ | ----------------------------- |
| Watcher / external rename-move-add-delete-enable-disable                | Disk Reconcile                      | Yes                            | Yes                           |
| Window refocus / first Mods entry / game switch hydrate / manual repair | Disk Reconcile                      | Yes when runtime state changed | Yes                           |
| `write_mod_ini` / `update_mod_info`                                     | Disk Reconcile (`InternalMutation`) | Yes                            | Yes                           |
| Explicit toggle / rename / move / delete mod from UI                    | Explicit runtime mutation service   | Yes                            | No                            |
| Explicit restore from trash                                             | Watcher + Disk Reconcile            | Yes if runtime state changed   | Yes                           |
| Thumbnail-only mutation                                                 | Disk Reconcile (`InternalMutation`) | No                             | Yes                           |

### Security & Privacy

- **Corridor Enforcement**: A Safe Mode collection query strictly appends `AND is_safe_context = ?`. An unsafe collection is mathematically impossible to apply while the frontend is in Safe Mode.
- **SSoT Validation**: The `save_collection` command strictly validates `Path::exists()` before committing to DB, preventing ghost entries from creeping into long-term storage.

---

## 4. Dependencies

- **Blocked by**: Epic 13 (Core Mod Ops - `rename_mod`), Epic 14 (OperationLock), Epic 28 (File Watcher), Epic 30 (Safe Mode - `switch_mode` uses these APIs), Epic 11 (Folder Listing & Classification semantics reused by collection preview trees).
- **Blocks**: Epic 35 (Smart Randomizer - uses collections backend to commit proposals).
