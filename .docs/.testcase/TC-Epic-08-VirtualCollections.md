# Test Case Scenarios: Epic 8 - Virtual Collections

**Objective:** Validate collection CRUD, preset apply/undo, export/import JSON, operation lock, context isolation, Safe Mode awareness, and `collection_items` schema.

**Ref:** [epic8-collection.md](file:///e:/Dev/EMMM2NEW/.docs/epic8-collection.md) | TRD §2.3, §3.6

---

## 1. Functional Test Cases (Positive)

### US-8.1: Loadout CRUD & Apply

| ID            | Title                 | Pre-Condition                   | Steps                                              | Expected Result                                                      | Post-Condition  | Priority |
| :------------ | :-------------------- | :------------------------------ | :------------------------------------------------- | :------------------------------------------------------------------- | :-------------- | :------- |
| **TC-8.1-01** | **Create Collection** | - 5 mods selected.              | 1. Click "Create Preset".<br>2. Name "Abyss Team". | - Saved to `collections` table.<br>- Members in `collection_items`.  | Preset visible. | High     |
| **TC-8.1-02** | **Apply Preset**      | - ModA active, Preset has ModB. | 1. Click "Apply Abyss Team".<br>2. Confirm modal.  | - ModA disabled.<br>- ModB enabled.<br>- Snapshot captured for undo. | Swapped.        | High     |
| **TC-8.1-03** | **Undo Apply**        | - Just applied preset.          | 1. Click "Undo" on toast (5s).                     | - All mods revert to exact previous state.<br>- Snapshot consumed.   | Reversed.       | High     |
| **TC-8.1-04** | **Context Switch**    | - User in "Star Rail" mode.     | 1. Open collections panel.                         | - Only SR presets shown.<br>- Genshin presets hidden.                | Filtered.       | High     |

### US-8.2: Export/Import

| ID            | Title                     | Pre-Condition          | Steps            | Expected Result                                                                        | Post-Condition | Priority |
| :------------ | :------------------------ | :--------------------- | :--------------- | :------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-8.2-01** | **Export as JSON**        | - "Abyss Team" preset. | 1. Click Export. | - JSON file saved with collection name, mod names, and metadata.<br>- Portable format. | File created.  | Medium   |
| **TC-8.2-02** | **Import on New Machine** | - Exported JSON file.  | 1. Import JSON.  | - Preset created.<br>- Mods matched by name/hash.<br>- Missing mods flagged.           | Imported.      | Medium   |

---

## 2. Negative Test Cases (Error Handling)

### US-8.1: Apply Failures

| ID            | Title                     | Pre-Condition                     | Steps            | Expected Result                                                     | Post-Condition   | Priority |
| :------------ | :------------------------ | :-------------------------------- | :--------------- | :------------------------------------------------------------------ | :--------------- | :------- |
| **NC-8.1-01** | **Missing Members**       | - Mod in preset was deleted.      | 1. Apply Preset. | - Warning: "ModX not found".<br>- Apply remaining valid mods.       | Partial success. | High     |
| **NC-8.1-02** | **Conflict in Preset**    | - Preset has Raiden A + Raiden B. | 1. Apply.        | - Error: "Conflict: A vs B".<br>- User selects which to keep.       | Resolved.        | High     |
| **NC-8.1-03** | **File Locked**           | - Mod locked by other app.        | 1. Apply.        | - Error: "Failed to toggle X".<br>- Transaction rollback.           | State safe.      | High     |
| **NC-8.1-04** | **Context Mismatch**      | - Genshin preset, user in SR.     | 1. Try to apply. | - Action blocked.<br>- Toast: "Preset belongs to different game".   | Blocked.         | High     |
| **NC-8.1-05** | **Operation Lock Active** | - Another operation running.      | 1. Click Apply.  | - Toast: "Operation in progress".<br>- Blocked until lock released. | Queued.          | High     |

---

## 3. Edge Cases & Stability

| ID          | Title                       | Simulation Step                                     | Expected Handling                                                                          | Priority |
| :---------- | :-------------------------- | :-------------------------------------------------- | :----------------------------------------------------------------------------------------- | :------- |
| **EC-8.01** | **Cross-Game DB Hack**      | 1. Modify DB: GIMI preset in SRMI context.          | - Application validates `game_id`.<br>- Block execution: "Invalid Game ID".                | Medium   |
| **EC-8.02** | **Safe Mode + NSFW Preset** | 1. Safe Mode ON.<br>2. Apply preset with NSFW mods. | - Warning: "Contains NSFW mods. Switch mode or skip".<br>- NSFW items excluded from apply. | High     |
| **EC-8.03** | **Rapid Apply/Undo**        | 1. Apply → Undo → Apply → Undo (fast).              | - Operation Lock prevents overlap.<br>- Wait for completion before next.                   | High     |
| **EC-8.04** | **Empty Collection**        | 1. Collection has 0 items.                          | - Apply button disabled.<br>- Delete button still works.                                   | Low      |
| **EC-8.05** | **Double Apply Click**      | 1. Click Apply twice rapidly.                       | - Operation Lock prevents duplicate.<br>- Only 1 execution.                                | High     |

---

## 4. Technical Metrics

| ID          | Metric               | Threshold  | Method                                         |
| :---------- | :------------------- | :--------- | :--------------------------------------------- |
| **TM-8.01** | **Snapshot Capture** | **< 10ms** | DB state capture before apply.                 |
| **TM-8.02** | **Apply I/O**        | **< 2s**   | 50 mod swap (disk + DB).                       |
| **TM-8.03** | **Snapshot Size**    | **< 10KB** | Undo JSON (only `mod_id` + `previous_status`). |

---

## 5. Data Integrity

| ID          | Object                 | Logic                                                                                  |
| :---------- | :--------------------- | :------------------------------------------------------------------------------------- |
| **DI-8.01** | **Snapshot Schema**    | JSON blob: `[{ mod_id, previous_status }]` for restore. Max 1 undo level.              |
| **DI-8.02** | **`collection_items`** | Table name is `collection_items` (not `collection_members`). FK cascade on mod delete. |
| **DI-8.03** | **`is_safe_context`**  | `collections` table has `is_safe_context BOOLEAN DEFAULT 0` per TRD.                   |
| **DI-8.04** | **info.json Sync**     | `preset_name` field written to mod's `info.json` for portability.                      |
