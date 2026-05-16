# Epic 14: Bulk Operations & Selection

## 1. Executive Summary

- **Problem Statement**: Power users managing large mod libraries need to toggle, move, or delete dozens of mods at once — doing so one at a time is impractical and error-prone; a single collision or lock mid-batch should not silently abort all remaining operations.
- **Proposed Solution**: A multi-select system (checkbox hover, shift-click range, ctrl-click) with a `BulkActionBar` overlay, plus backend batch switch/delete/move commands that stream per-item progress events, hold a global `OperationLock`, and report partial failures without aborting the entire batch.
- **Success Criteria**:
  - Shift-click range selection of 100 items applies in ≤ 50ms (pure client-side index intersection).
  - `BulkActionBar` renders in ≤ 100ms after first item is selected.
  - Bulk toggle of 100 mods completes in ≤ 5s on SSD (sequential renames with watcher suppression).
  - Progress events stream to frontend at ≥ 1 update/s for batches ≥ 20 items.
  - Partial collision in bulk move surfaces a `ResolverDialog` for the conflicting item — remaining items continue processing.

---

## 2. User Experience & Functionality

### User Stories

#### US-14.1: Multi-Select Mods

As a user, I want to select multiple mods using checkboxes or keyboard modifiers, so that I can apply bulk actions over a set without clicking each individually.

| ID        | Type        | Criteria                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-14.1.1 | ✅ Positive | Given an active grid, when hovering a card or during selection, a checkbox appears in the **top-right corner**; clicking it adds the item to `selectedItems[]` in Zustand      |
| AC-14.1.2 | ✅ Positive | Given `selectedItems.length ≥ 1`, then the `BulkActionBar` slides in at the bottom of the grid in ≤ 100ms, showing count and available actions                                 |
| AC-14.1.3 | ✅ Positive | Given I shift-click a second item after selecting the first, then all items between them (by current sort order) are added to the selection in ≤ 50ms                          |
| AC-14.1.4 | ✅ Positive | Given the `Escape` key is pressed while items are selected, all selection is cleared immediately and the action bar hides                                                      |
| AC-14.1.5 | ❌ Negative | Given a user attempts to select an item currently being processed by a background operation (item in `pendingOps` set), that item's checkbox is disabled — no selection occurs |
| AC-14.1.6 | ⚠️ Edge     | Given the user navigates to a different object or sub-path while a selection is active, then `selectedItems` is cleared immediately (no cross-folder batch)                    |

---

#### US-14.2: Bulk Action Bar

As a user, I want a focused bulk action toolbar to appear when I have items selected, so that I can clearly see my selection count and invoke mass actions.

| ID        | Type        | Criteria                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-14.2.1 | ✅ Positive | Given ≥ 1 selected items, the `BulkActionBar` shows: "{N} selected", "Enable All", "Disable All", "Move to Object…", "Delete Selected", "Deselect All"                                          |
| AC-14.2.2 | ✅ Positive | Given I click "Deselect All", then `selectedItems` resets to `[]` and the BulkActionBar slides out in ≤ 100ms                                                                                   |
| AC-14.2.3 | ❌ Negative | Given an unhandled error occurs during a bulk action, then the BulkActionBar resets to its idle state and shows "Operation failed — {N} errors" toast; it does not get stuck in a loading state |

---

#### US-14.3: Bulk Move

As a user, I want to move multiple selected mods to a specific Object in one action, so that I can re-categorize large unorganized sets quickly.

| ID        | Type        | Criteria                                                                                                                                                                                                                                       |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-14.3.1 | ✅ Positive | Given N selected FolderGrid mods, when I click "Move to Object…" and pick a target object plus root/existing subfolder target, then `move_mods_to_object` moves the selected folder paths under one guarded batch operation                    |
| AC-14.3.2 | ✅ Positive | Given all items complete successfully, then runtime path rewrites and centralized refresh descriptors update FolderGrid/ObjectList/Preview without raw feature-level query invalidation                                                        |
| AC-14.3.3 | ❌ Negative | Given 1 of N folders collides with an existing name in the target, then a `ResolverDialog` surfaces for that specific item (Skip / Rename / Overwrite); the remaining N-1 items continue processing — partial failure does NOT abort the batch |
| AC-14.3.4 | ⚠️ Edge     | Given a second bulk move is triggered while the first is in progress, then the "Move to Object…" button is disabled (grayed) during the in-flight operation — only one batch op per game path at a time (`OperationLock` enforced)             |

---

### Non-Goals

- No undo stack for bulk operations (unlike single toggle).
- No regex-based "select by pattern" in this phase — selection is manual (checkbox + shift-click + ctrl-click).
- Batch operations are sequential (not concurrent per-item) to avoid filesystem race conditions.
- No progress bar for batches < 5 items — single toast on completion suffices.

---

## 3. Technical Specifications

### Architecture Overview

```
Frontend:
  selectedItems: Set<string>  ← folder_paths, in Zustand

  BulkActionBar (portal-mounted, slides in when selectedItems.size > 0)
    ├── "Enable All" → commands.bulkToggle({ paths: [...selectedItems], enable: true })
    ├── "Disable All" → commands.bulkToggle({ paths: [...selectedItems], enable: false })
    ├── "Move to Object…" → MoveToObjectDialog → commands.moveModsToObject({ folderPaths, targetObjectId, targetSubpath, status })
    └── "Delete Selected" → ConfirmDialog → commands.bulkDelete({ paths: [...selectedItems] })

Backend:
  bulk_switch_mods(game_id, paths: Vec<String>, enable: bool) → BulkResult
    └── acquire OperationLock → activate WatcherSuppression(paths)
        → for path in paths: switch_item_internal(path, enable) → emit_event('bulk-progress', {i, total})
        → return BulkResult { success, failed: Vec<{path, error}> }

  move_mods_to_object(game_id, paths, target_object_id, target_subpath, status) → BulkResult
    └── same pattern: validates root/subfolder target, returns per-item failures + runtime path rewrites

  bulk_delete_mods(game_id, paths) → BulkResult
    └── same pattern: trash per item (app trash) → stream progress ('bulk-progress')
```

### Integration Points

| Component            | Detail                                                                                                                                    |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Selection State      | `useAppStore.gridSelection: Set<string>` (folder_paths).                                                                                  |
| `OperationLock`      | Per `game_id` `Arc<Mutex<()>>` — bulk ops and single ops share the same lock.                                                             |
| Progress Events      | Tauri `Window::emit("bulk-progress", {current, total, label, active})`.                                                                   |
| Runtime Refresh      | Bulk mutation results map to centralized runtime descriptors / `WorkspaceImpact`; feature code does not call raw query invalidation APIs. |
| `WatcherSuppression` | All paths in the batch added to suppression set before any op, removed after last completes                                               |

### Security & Privacy

- **All paths in `paths[]` are validated individually** via `canonicalize()` + `starts_with(mods_path)` before batch starts — one invalid path does not start the batch.
- **`OperationLock` prevents concurrent bulk + single ops** on the same game path — no TOCTOU race between a bulk toggle and a single rename.
- **`WatcherSuppression` covers all batch paths atomically** — added as a set before the first op, not incrementally.

---

## 4. Dependencies

- **Blocked by**: Epic 12 (Folder Grid — selection UI), Epic 13 (Core Mod Ops — reuses toggle/move/delete internals), Epic 28 (File Watcher — WatcherSuppression).
- **Blocks**: Epic 15 (Explorer Interactions — DnD multi-item drop triggers `move_mods_to_object`).
