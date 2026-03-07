# Epic 14: Bulk Operations & Selection

## 1. Executive Summary

- **Problem Statement**: Power users managing large mod libraries need to toggle, move, or delete dozens of mods at once — doing so one at a time is impractical and error-prone; a single collision or lock mid-batch should not silently abort all remaining operations.
- **Proposed Solution**: A multi-select system (checkbox hover, shift-click range, drag-marquee) with a `BulkActionBar` overlay, plus backend batch commands (`bulk_toggle`, `bulk_move`, `bulk_delete`) that stream per-item progress events, hold a global `OperationLock`, and report partial failures without aborting the entire batch.
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

| ID        | Type        | Criteria                                                                                                                                                                               |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-14.1.1 | ✅ Positive | Given an active grid, when hovering a card, a checkbox appears (visible on hover, always visible when any item is selected); clicking it adds the item to `selectedItems[]` in Zustand |
| AC-14.1.2 | ✅ Positive | Given `selectedItems.length ≥ 1`, then the `BulkActionBar` slides in at the bottom of the grid in ≤ 100ms, showing count and available actions                                         |
| AC-14.1.3 | ✅ Positive | Given I shift-click a second item after selecting the first, then all items between them (by current sort order) are added to the selection in ≤ 50ms                                  |
| AC-14.1.4 | ❌ Negative | Given a user attempts to select an item currently being processed by a background operation (item in `pendingOps` set), that item's checkbox is disabled — no selection occurs         |
| AC-14.1.5 | ⚠️ Edge     | Given the user navigates to a different object or sub-path while a selection is active, then `selectedItems` is cleared immediately (no cross-folder batch)                            |

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
| AC-14.3.1 | ✅ Positive | Given N selected mods, when I click "Move to Object…" and pick a target, then `bulk_move(paths[], target_object_path)` is invoked; each folder is moved sequentially with a streamed progress event per item                                   |
| AC-14.3.2 | ✅ Positive | Given all items complete successfully, then both the source `['folders', gameId, srcPath]` and target `['folders', gameId, targetPath]` caches invalidate; `selectedItems` clears                                                              |
| AC-14.3.3 | ❌ Negative | Given 1 of N folders collides with an existing name in the target, then a `ResolverDialog` surfaces for that specific item (Skip / Rename / Overwrite); the remaining N-1 items continue processing — partial failure does NOT abort the batch |
| AC-14.3.4 | ⚠️ Edge     | Given a second bulk move is triggered while the first is in progress, then the "Move to Object…" button is disabled (grayed) during the in-flight operation — only one batch op per game path at a time (`OperationLock` enforced)             |

---

### Non-Goals

- No undo stack for bulk operations; Trash handles delete recovery.
- No regex-based "select by pattern" in this phase — selection is manual (checkbox + shift-click + marquee).
- Batch operations are sequential (not concurrent per-item) to avoid filesystem race conditions.
- No progress bar for batches < 5 items — single toast on completion suffices.

---

## 3. Technical Specifications

### Architecture Overview

```
Frontend:
  selectedItems: Set<string>  ← folder_paths, in Zustand

  BulkActionBar (portal-mounted, slides in when selectedItems.size > 0)
    ├── "Enable All" → invoke('bulk_toggle', { paths: [...selectedItems], enable: true })
    ├── "Disable All" → invoke('bulk_toggle', { paths: [...selectedItems], enable: false })
    ├── "Move to Object…" → ObjectPickerModal → invoke('bulk_move', { paths, targetObjectPath })
    └── "Delete Selected" → ConfirmDialog → invoke('bulk_delete', { paths: [...selectedItems] })

Backend:
  bulk_toggle(game_id, paths: Vec<String>, enable: bool) → BulkResult
    └── acquire OperationLock → activate WatcherSuppression(paths)
        → for path in paths: toggle_mod_internal(path, enable) → emit_event('bulk:progress', {i, total})
        → return BulkResult { success, failed: Vec<{path, error}> }

  bulk_move(game_id, paths, target_object_path) → BulkResult
    └── same pattern: check collision per item → move or pause for ResolverDialog

  bulk_delete(game_id, paths) → BulkResult
    └── same pattern: trash::delete per item → stream progress
```

### Integration Points

| Component            | Detail                                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------------------------ |
| Selection State      | `useAppStore.selectedItems: Set<string>` (folder_paths)                                                      |
| `OperationLock`      | Per `game_id` `Arc<Mutex<()>>` — bulk ops and single ops share the same lock                                 |
| Progress Events      | Tauri `Window::emit("bulk:progress", {current, total})` → `listen` in React                                  |
| Cache Invalidate     | `queryClient.invalidateQueries(['folders', gameId])` after `BulkResult` resolves (both src and target paths) |
| `WatcherSuppression` | All paths in the batch added to suppression set before any op, removed after last completes                  |

### Security & Privacy

- **All paths in `paths[]` are validated individually** via `canonicalize()` + `starts_with(mods_path)` before batch starts — one invalid path does not start the batch.
- **`OperationLock` prevents concurrent bulk + single ops** on the same game path — no TOCTOU race between a bulk toggle and a single rename.
- **`WatcherSuppression` covers all batch paths atomically** — added as a set before the first op, not incrementally.

---

## 4. Dependencies

- **Blocked by**: Epic 12 (Folder Grid — selection UI), Epic 13 (Core Mod Ops — reuses toggle/move/delete internals), Epic 28 (File Watcher — WatcherSuppression).
- **Blocks**: Epic 15 (Explorer Interactions — DnD multi-item drop triggers bulk_move).
