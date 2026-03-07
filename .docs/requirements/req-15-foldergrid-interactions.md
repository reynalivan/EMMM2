# Epic 15: Grid Interactions & Context Menus

## 1. Executive Summary

- **Problem Statement**: Beyond single-click toggle, users need rich contextual actions on mod cards (rename, delete, open in Explorer, paste thumbnail) from a right-click menu — and a drag-selection marquee and drag-to-objectlist for rapid re-categorization.
- **Proposed Solution**: A right-click context menu (custom, not browser default) on cards and grid background, a lasso-select marquee drawn via CSS overlay with auto-scroll at edges, a DnD system (`dnd-kit`) connecting grid cards as drag sources to objectlist Object rows as drop targets, and a clipboard image paste-to-thumbnail action.
- **Success Criteria**:
  - Context menu appears within ≤ 50ms of right-click (positioned correctly within viewport bounds).
  - Thumbnail paste (clipboard → `preview.png`) completes in ≤ 500ms.
  - Lasso selection of 50 cards in a 200-card grid updates `selectedItems` in ≤ 100ms.
  - DnD overlay renders following the cursor within ≤ 16ms (one frame at 60fps).
  - Drop-on-invalid-zone animates back to source position in ≤ 200ms (no items moved on bad drop).

---

## 2. User Experience & Functionality

### User Stories

#### US-15.1: Folder Context Menu

As a user, I want to right-click a mod card to access advanced actions, so that I can open in Explorer, rename, delete, or paste a thumbnail without a dedicated settings screen.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                             |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-15.1.1 | ✅ Positive | Given a right-click on a mod card, then a context menu appears in ≤ 50ms, positioned within the viewport, with options: "Open in Explorer", "Edit Metadata", "Sync with DB", "Rename", "Toggle", "Favorite", "Paste Thumbnail", "Import Thumbnail...", "Move to Object...", "Delete" |
| AC-15.1.2 | ✅ Positive | Given I click "Paste Thumbnail" and the OS clipboard contains an image, then the raw image bytes are read via `tauri-plugin-clipboard-manager`, saved as `preview.png` in the mod folder, and the card thumbnail updates in ≤ 500ms                                                  |
| AC-15.1.3 | ❌ Negative | Given the clipboard contains text or a non-image file, then "Paste Thumbnail" is grayed out/disabled — clicking it does nothing and shows no error                                                                                                                                   |
| AC-15.1.4 | ⚠️ Edge     | Given I click "Open in Explorer" on a folder deleted externally 1s before the click, then the OS shell command fails; a toast shows "Folder no longer exists" — no crash                                                                                                             |
| AC-15.1.5 | ✅ Positive | Given I click "Favorite/Unfavorite", then the `is_favorite` flag is toggled simultaneously in the database and written to the mod's `info.json`                                                                                                                                      |
| AC-15.1.6 | ✅ Positive | Given I click "Import Thumbnail...", a file dialog opens (filtering PNG/JPG/WebP); on selection, the image is copied and saved as `preview_custom.png`                                                                                                                               |
| AC-15.1.7 | ✅ Positive | Given I click "Move to Object...", the `MoveToObjectDialog` opens, providing a searchable object list and a status selector (Set Disabled / Only Enable This / Keep Status) to assign the mod to a new object path                                                                   |
| AC-15.1.8 | ✅ Positive | Given I click "Sync with DB", then `match_object_with_db` uses the folder name to find a MasterDB entry and opens `SyncConfirmModal` for diff preview before applying metadata and renaming the folder                                                                               |

---

#### US-15.2: Grid Background Context Menu

As a user, I want to right-click empty grid space to access global grid actions, so that I can refresh, select all, or open the current folder path.

| ID        | Type        | Criteria                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-15.2.1 | ✅ Positive | Given a right-click precisely on the grid background (not on any card), then a context menu shows: "Refresh", "Select All", "Open Folder in Explorer"             |
| AC-15.2.2 | ✅ Positive | Given "Refresh" is clicked, then `queryClient.invalidateQueries(['folders', gameId, subPath])` triggers a re-fetch — the grid reloads without a full page refresh |
| AC-15.2.3 | ❌ Negative | Given "Select All" is clicked when the grid has 0 items, then the action is disabled (menu item is grayed out) — `selectedItems` stays empty                      |

---

#### US-15.3: Drag-Marquee (Lasso) Selection

As a user, I want to click-drag on empty grid space to draw a selection rectangle over mod cards, so that I can select multiple items without holding modifier keys.

| ID        | Type        | Criteria                                                                                                                                                                                                                                         |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-15.3.1 | ✅ Positive | Given I mousedown on empty grid space and drag, then a semi-transparent CSS rectangle overlay is drawn following the cursor                                                                                                                      |
| AC-15.3.2 | ✅ Positive | Given the rectangle intersects any mod card's bounding box, that card is added to `selectedItems[]` in real-time as the user drags                                                                                                               |
| AC-15.3.3 | ⚠️ Edge     | Given the drag extends beyond the visible grid container (cursor at bottom of grid), then the container auto-scrolls downward at a rate proportional to the cursor's distance below the boundary — selection continues through virtualized items |

---

#### US-15.4: Drag-and-Drop to ObjectList Object

As a user, I want to drag one or more selected mod cards onto an Object in the objectlist, so that I can re-categorize mods rapidly without using the bulk move dialog.

| ID        | Type        | Criteria                                                                                                                                                                                                            |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-15.4.1 | ✅ Positive | Given I start dragging a mod card (or multiple selected cards), then a `DragOverlay` floats under the cursor showing "{N} folder(s)" label — the source cards show a dimmed "dragging" state                        |
| AC-15.4.2 | ✅ Positive | Given I drop the overlay on a valid Object row in the objectlist, then `bulk_move(selectedPaths, targetObjectPath)` is invoked (Epic 14); `selectedItems` clears on success                                            |
| AC-15.4.3 | ❌ Negative | Given I drop the overlay on an invalid zone (TopBar, BulkActionBar, PreviewPanel), then the overlay animates back to the source position (`dropAnimation` in dnd-kit) in ≤ 200ms — no file operations are triggered |
| AC-15.4.4 | ⚠️ Edge     | Given I drop the overlay on the same Object the mods are already in (self-drop), then the move is a no-op — `bulk_move` short-circuits, no disk rename, no toast                                                    |

---

### Non-Goals

- No native OS drag (file manager drag-in from File Explorer) — only intra-app DnD via `dnd-kit`.
- No animated card thumbnail zoom on hover (performance risk with 1000+ cards).
- No right-click context menu on the ObjectList's ObjectList rows — that is Epic 10 (Object CRUD).
- Clipboard paste supports only raster images (PNG/JPEG); SVG and WebP are not supported in this phase.

---

## 3. Technical Specifications

### Architecture Overview

```
Context Menu (custom, portal-mounted)
  ├── CardContextMenu (appears on card right-click)
  │   ├── invokeShellOpen(folderPath)     → invoke('open_in_explorer', { path })
  │   ├── → trigger rename modal          (Epic 13)
  │   ├── → trigger delete confirm        (Epic 13)
  │   └── pasteClipboardImage(folderPath) → invoke('paste_thumbnail', { path })
  └── GridContextMenu (appears on grid BG right-click)
      ├── invalidateQueries(['folders', ...])
      ├── selectAll() → setSelectedItems(allFolderPaths)
      └── invokeShellOpen(currentSubPath)

Lasso Selection
  └── onMouseDown(emptySpace) → track {startX, startY}
      → onMouseMove → draw <div> overlay (position: absolute)
      → intersect overlay rect with each virtualizer item rect → add to selectedItems
      → onMouseUp → cleanup overlay

DnD (dnd-kit)
  ├── DndContext > DragOverlay
  ├── useDraggable on ModCard (draggable id = folder_path)
  └── useDroppable on ObjectRow (droppable id = object_folder_path)
      → onDragEnd: if over valid target → bulk_move(selectedItems || [activeId], over.id)
                   else → no-op (dropAnimation plays)
```

### Integration Points

| Component       | Detail                                                                                                                                 |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Context Menu    | Custom React component — `onContextMenu` event captured on card and grid background, `event.preventDefault()` to suppress browser menu |
| Clipboard       | `tauri-plugin-clipboard-manager` — `readImageBase64()` → decode → `writeFile('preview.png')`                                           |
| Shell Open      | `invoke('open_in_explorer', { path })` → Rust `Command::new("explorer").arg(path).spawn()`                                             |
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
