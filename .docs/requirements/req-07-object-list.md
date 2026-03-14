# Epic 07: Object List

## 1. Executive Summary

- **Problem Statement**: The objectlist object list needs to handle thousands of game objects (characters, weapons, UI elements) without freezing the UI — and must keep mod counts (total/enabled) accurate in real-time as users toggle mods in the grid.
- **Proposed Solution**: A virtualized object list powered by `@tanstack/react-virtual`, with optimistic count updates on mod toggle, drag-and-drop support for re-categorizing mods across objects, and a forced selection reset on game switch.
- **Success Criteria**:
  - ObjectList renders ≥ 1,000 items without dropping below 60fps, verified via React DevTools Profiler Flamegraph.
  - Scroll through 1,000+ items: no blank flicker lasting > 1 frame (16ms).
  - Enabled/total count badge updates within ≤ 50ms of a mod toggle (optimistic update, no refetch required).
  - Bulk enable/disable of 100 mods batches all objectlist count updates into a single render tick.
  - Drag-and-drop folder move completes (disk write + cache invalidate) in ≤ 500ms for a single mod folder.

---

## 2. User Experience & Functionality

### User Stories

#### US-07.1: Virtualized Rendering

As a performance-conscious user, I want the objectlist to stay responsive with thousands of objects, so that the app never freezes during navigation.

| ID        | Type        | Criteria                                                                                                                                                                          |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.1.1 | ✅ Positive | Given a list of ≥ 1,000 objects, when the objectlist renders, then only the DOM nodes for visually visible rows are created (virtualized) — maximum 30 DOM nodes at any time         |
| AC-07.1.2 | ✅ Positive | Given rapid scroll through ≥ 1,000 items, then scroll is ≥ 60fps — no blank white flickers lasting > 1 frame (16ms)                                                               |
| AC-07.1.3 | ❌ Negative | Given resource-constrained hardware (< 4GB RAM), when scrolling violently, the list drops frames visually before freezing the main JS thread — UI remains responsive to clicks    |
| AC-07.1.4 | ⚠️ Edge     | Given a dynamic window resize that changes objectlist height by > 50%, the virtualized list recalculates visible bounds within ≤ 100ms without throwing an out-of-bounds index error |

---

#### US-07.2: Object Selection & Navigation

As a user, I want to click an object in the objectlist to view its mod folders in the center grid, so that I can manage mods for that specific character or entity.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-07.2.1 | ✅ Positive | Given a click on an object row, then `selectedObjectId` in Zustand updates within ≤ 16ms and the selected row shows a highlight indicator                                                                                                                                      |
| AC-07.2.2 | ✅ Positive | Given a new `selectedObjectId`, then the center FolderGrid query (`['folders', gameId, objectId]`) invalidates and the new mod list loads within ≤ 200ms from DB                                                                                                               |
| AC-07.2.3 | ❌ Negative | Given an object that was deleted by a background process while the objectlist was cached, when the user clicks it, then the stale row is removed from the list without an error toast — the action is silently swallowed and the selection remains on the previously valid object |
| AC-07.2.4 | ⚠️ Edge     | Given the user switches the active game, then `selectedObjectId` is immediately cleared to `null` before the new game's object list loads — preventing a cross-game `objectId` in flight                                                                                       |

---

#### US-07.3: Dynamic Enabled Counts

As a user, I want to see real-time enabled/total mod counts on each object row, so that I know at a glance which characters have active mods.

| ID        | Type        | Criteria                                                                                                                                                                                                                                           |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.3.1 | ✅ Positive | Given an object row, then it displays a badge showing `{enabled}/{total}` mod folder counts, derived from the `get_objects` query aggregation                                                                                                      |
| AC-07.3.2 | ✅ Positive | Given the user toggles a mod in the grid (enable/disable), then the objectlist badge for that object updates within ≤ 50ms via optimistic state mutation — before the backend confirms                                                                |
| AC-07.3.3 | ❌ Negative | Given all mods under an object are disabled (enabled count = 0), then the badge is styled as dim/inactive and the row is not highlighted as "active"                                                                                               |
| AC-07.3.4 | ⚠️ Edge     | Given a bulk toggle of 100 mods in one action, then all 100 objectlist count increments/decrements are batched into a single React render tick via `unstable_batchedUpdates` or a Zustand immer batch — no frame drops from 100 individual re-renders |

---

#### US-07.4: Drag-and-Drop Mod Re-Categorization

As a user, I want to drag a mod folder from the center grid onto a different objectlist object, so that I can re-categorize it without using the context menu.

| ID        | Type        | Criteria                                                                                                                                                                                                  |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.4.1 | ✅ Positive | Given the user drags a mod card over a objectlist object row, then that row shows a drop-target border highlight within ≤ 50ms of hover                                                                      |
| AC-07.4.2 | ✅ Positive | Given a drop on a target object, then `move_mod(src_path, target_object_path)` is invoked; the folder is physically moved on disk within ≤ 500ms                                                          |
| AC-07.4.3 | ✅ Positive | Given a successful disk move, then `['folders', gameId, srcObjectId]` and `['folders', gameId, targetObjectId]` React Query caches both invalidate and the grid refreshes within ≤ 200ms                  |
| AC-07.4.4 | ❌ Negative | Given the target object's folder already contains a mod with the same name, then the `move_mod` command returns a `CollisionError`, no file is moved, and a conflict resolution dialog is shown (Epic 39) |
| AC-07.4.5 | ⚠️ Edge     | Given the user drags to a non-visible virtualized row (forcing auto-scroll at the list edge), then the drop coordinates correctly map to the virtualized item index — not the visible DOM position        |

---

### Non-Goals

- No inline renaming of objects directly in the objectlist.
- No per-object thumbnail in the objectlist rows (icon only, no preview image).
- No multi-object selection or drag-group operations.
- No user-created "custom objects" or tags beyond what the `GameSchema` defines.
- No network-fetching of object metadata; all data comes from the local SQLite `objects` table.

---

## 3. Technical Specifications

### Architecture Overview

```
ObjectList (objectlist component)
  ├── useObjects(gameId, filters) → React Query → invoke('get_objects_cmd')
  └── VirtualizedList (@tanstack/react-virtual)
      └── ObjectRow (per item)
          ├── name, thumbnailUri
          ├── Badge: {enabled}/{total}  ← from aggregated query field
          ├── onClick → setSelectedObjectId(objectId)
          └── DroppableArea (dnd-kit droppable)

Backend: get_objects_cmd(game_id, filter) →
  SELECT o.*, COUNT(f.*), SUM(CASE WHEN f.is_enabled THEN 1 ELSE 0 END)
  FROM objects o LEFT JOIN folders f ON f.object_id = o.id
  WHERE o.game_id = ?
  GROUP BY o.id
```

### Integration Points

| Component         | Detail                                                                             |
| ----------------- | ---------------------------------------------------------------------------------- |
| Data Source       | `invoke('get_objects_cmd', { gameId, filter })` → `Vec<ObjectWithCounts>`          |
| Virtualization    | `@tanstack/react-virtual` — `useVirtualizer({ count, estimateSize: () => 48 })`    |
| DnD               | `dnd-kit` — `useDraggable` (FolderCard) + `useDroppable` (ObjectRow)               |
| Optimistic Update | `queryClient.setQueryData(['objects', gameId], updater)` on mod toggle             |
| Move Command      | `invoke('move_mod', { srcPath, targetObjectPath })` — atomic rename on disk        |
| Batch Render      | React 18 automatic batching — all count updates within one async event are batched |

### Security & Privacy

- **Read-only objectlist** — the object list itself displays data but does not mutate any filesystem path or DB record; all mutations go through specific IPC commands (`move_mod`) with validated paths.
- **Safe Mode filter**: ObjectList ALWAYS shows all objects regardless of safe mode (to prevent the navigation pane from disappearing). Instead of removing objects from the list, counts are purely based on Mutually Exclusive Corridors (Safe Mode ONLY counts safe objects, Unsafe Mode ONLY counts unsafe objects; out-of-corridor items show `0/0`).

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap), Epic 02 (Game Management — `activeGameId`), Epic 05 (Workspace Layout — panel shell), Epic 06 (ObjectList — rendering container), Epic 09 (Object Schema — category grouping).
- **Blocks**: Epic 12 (Folder Grid — listens to `selectedObjectId`), Epic 15 (Explorer Interactions — DnD source), Epic 40 (Metadata Actions — object pinning).
