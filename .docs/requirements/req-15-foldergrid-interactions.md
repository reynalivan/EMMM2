# Epic 15: Grid Interactions & Context Menus

## 1. Executive Summary

- **Problem Statement**: Beyond single-click toggle, users need rich contextual actions on mod cards (rename, delete, open in Explorer, paste thumbnail) from a right-click menu — and a drag-selection marquee and drag-to-objectlist for rapid re-categorization.
- **Proposed Solution**: A right-click context menu (custom, not browser default) on cards and grid background, standardized Shift-click and Ctrl-click selection, and a "Move to Object" dialog for re-categorization. Menu structure is resolved by shared policy, while imperative actions (Explorer open, thumbnail paste/import, move, sync) are dispatched through dedicated action hooks.
- **Success Criteria**:
  - [x] Context menu appears within ≤ 50ms of right-click (positioned correctly within viewport bounds).
  - [x] Thumbnail paste (clipboard → `preview.png`) completes in ≤ 500ms.
  - [x] Multi-selection (Shift/Ctrl) of 50 cards in a 200-card grid updates `gridSelection` in ≤ 50ms.
  - [x] Delete and Rename flow triggers for all selected items (Bulk Delete / Rename Focused).

---

## 2. User Experience & Functionality

### User Stories

#### US-15.1: Folder Context Menu

As a user, I want to right-click a mod card to access advanced actions, so that I can open in Explorer, rename, delete, or paste a thumbnail without a dedicated settings screen.

| ID        | Type        | Criteria                                                                                                                                                                                                                            |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-15.1.1 | ✅ Positive | Given a right-click on a mod card, then a context menu appears in ≤ 50ms, with options: "Open in Explorer", "Rename", "Enable/Disable", "Favorite", "Paste Thumbnail", "Import Thumbnail...", "Move to Object...", "Delete"         |
| AC-15.1.2 | ✅ Positive | Given I click "Paste Thumbnail" and the OS clipboard contains an image, then the raw image bytes are read via `tauri-plugin-clipboard-manager`, saved as `preview.png` in the mod folder, and the card thumbnail updates in ≤ 500ms |
| AC-15.1.3 | ❌ Negative | Given the clipboard contains text or a non-image file, then "Paste Thumbnail" is grayed out/disabled — clicking it does nothing and shows no error                                                                                  |
| AC-15.1.4 | ⚠️ Edge     | Given I click "Open in Explorer" on a folder deleted externally 1s before the click, then the OS shell command fails; a toast shows "Folder no longer exists" — no crash                                                            |
| AC-15.1.5 | ✅ Positive | Given I click "Favorite/Unfavorite", then the `is_favorite` flag is toggled simultaneously in the database and written to the mod's `info.json`                                                                                     |
| AC-15.1.6 | ✅ Positive | Given I click "Import Thumbnail...", a file dialog opens (filtering PNG/JPG/WebP); on selection, the image is copied and saved as `preview_custom.png`                                                                              |
| AC-15.1.7 | ✅ Positive | Given I click "Move to Object...", the `MoveToObjectDialog` opens, providing a searchable object list and a status selector (Set Disabled / Only Enable This / Keep Status) to assign the mod to a new object path                  |
| AC-15.1.8 | ✅ Positive | Given I click "Sync with DB", then `match_object_with_db` uses the folder name to find a MasterDB entry and opens `SyncConfirmModal` for diff preview before applying metadata and renaming the folder                              |

---

#### US-15.2: Grid Background Context Menu

As a user, I want to right-click empty grid space to access global grid actions, so that I can refresh, select all, or open the current folder path.

| ID        | Type        | Criteria                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-15.2.1 | ✅ Positive | Given a right-click precisely on the grid background (not on any card), then a context menu shows: "Refresh", "Select All", "Open Folder in Explorer"             |
| AC-15.2.2 | ✅ Positive | Given "Refresh" is clicked, then the shared runtime refresh path publishes a folder/workspace refresh descriptor and the grid reloads without a full page refresh     |
| AC-15.2.3 | ❌ Negative | Given "Select All" is clicked when the grid has 0 items, then the action is disabled (menu item is grayed out) — `selectedItems` stays empty                      |

---

#### US-15.3: Drag-Marquee (Lasso) Selection

As a user, I want to click-drag on empty grid space to draw a selection rectangle over mod cards, so that I can select multiple items without holding modifier keys.

| ID        | Type        | Criteria                                                                                                                           |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| AC-15.3.1 | ✅ Positive | Given I mousedown on empty grid space and drag, then a semi-transparent CSS rectangle overlay is drawn following the cursor        |
| AC-15.3.2 | ✅ Positive | Given the rectangle intersects any mod card's bounding box, that card is added to `selectedItems[]` in real-time as the user drags |

#### US-15.3: Selection Logic

| ID        | Type        | Criteria                                                                                      |
| --------- | ----------- | --------------------------------------------------------------------------------------------- |
| AC-15.3.1 | ✅ Positive | Given a mod card, Ctrl+Click toggles its selection state without clearing others.             |
| AC-15.3.2 | ✅ Positive | Given a mod card, Shift+Click selects a range from the last selected item to the current one. |
| AC-15.3.3 | ✅ Positive | Click on empty grid background clears all selection.                                          |

---

#### US-15.4: Keyboard Navigation

| ID        | Type        | Criteria                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------ |
| AC-15.4.1 | ✅ Positive | Arrow keys move focus between cards in grid mode (handling columns correctly). |
| AC-15.4.2 | ✅ Positive | Enter key navigates into a folder or triggers default action.                  |
| AC-15.4.3 | ✅ Positive | Backspace key navigates up to the parent folder.                               |
| AC-15.4.4 | ✅ Positive | Delete key opens delete confirmation for focused/selected items.               |
| AC-15.4.5 | ✅ Positive | F2 key starts renaming for the focused item.                                   |

---

### Non-Goals

- Clipboard paste supports common raster formats via browser navigator.clipboard.

---

## 3. Technical Specifications

### Architecture Overview

```
Context Menu (custom, portal-mounted)
  ├── CardContextMenu (appears on card right-click)
  │   ├── buildModContextMenuItems(policy, callbacks)
  │   ├── useModContextMenuActions(folder)
  │   ├── handleToggleEnabled(folder)
  │   ├── handleRenameRequest(folder)
  │   ├── handleDeleteRequest(folder)
  │   └── handlePasteThumbnail(folder)
  └── Toolbar Refresh
      ├── publish runtime refresh descriptor

Keyboard Navigation:
  - Arrow keys: setFocusedId
  - Backspace: handleGoUp
  - F2: handleRenameRequest
  - Delete: handleDeleteRequest
```

### Integration Points

| Component       | Detail                                                                                                                                 |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Context Menu    | Custom React component — `onContextMenu` event captured on card and grid background, with policy-only menu descriptors and dedicated action hooks |
| Clipboard       | `tauri-plugin-clipboard-manager` — `readImageBase64()` → decode → `writeFile('preview.png')`                                           |
| Shell Open      | Explorer open action lives in mod action hooks; policy/menu builders do not call commands directly                                      |
| Lasso           | `useRef` for overlay `<div>`, `getBoundingClientRect()` intersection per virtual row rect                                              |
| DnD             | `@dnd-kit/core` `DndContext`, `useDraggable`, `useDroppable`, `DragOverlay`                                                            |
| Bulk Move (DnD) | Delegates to Epic 14 `bulk_move` on `onDragEnd`                                                                                        |

### Security & Privacy

- **Shell open path is validated** on the Rust side via `canonicalize()` + `starts_with(mods_path)` before `Command::new("explorer")` is called — prevents launching arbitrary paths.
- **Clipboard image is written directly to `folderPath/preview.png`** — path validated as above; no arbitrary write destination.
- **`event.preventDefault()` on `onContextMenu`** prevents any browser context menu or default browser action from interfering; the custom menu is the only visible result.
- **DnD drop target IDs (`over.id`)** are always validated against the `objects` DB table on the backend before `bulk_move` executes — a spoofed droppable ID is rejected.

---

## 4. Dependencies

- **Blocked by**: Epic 12 (Folder Grid — card rendering), Epic 13 (Core Mod Ops — rename/delete commands), Epic 14 (Bulk Ops — bulk_move for DnD), Epic 07 (Object List — objectlist droppable rows).
- **Blocks**: Nothing — this is a cross-cutting interaction layer consumed by the completed workspace.
