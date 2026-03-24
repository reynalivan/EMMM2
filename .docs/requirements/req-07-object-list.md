# Epic 07: Object List

## 1. Executive Summary

- **Problem Statement**: The objectlist object list needs to handle thousands of game objects (characters, weapons, UI elements) without freezing the UI — and must keep mod counts (total/enabled) accurate in real-time as users toggle mods in the grid.
- **Proposed Solution**: A virtualized object list powered by `@tanstack/react-virtual`, with accurate mod counts (restricted by active safety corridor), zone-aware drag-and-drop support (Toolbar: Auto, Row: Move, Status: Append), and a forced selection reset on game switch.
- **Success Criteria**:
  - ObjectList renders ≥ 1,000 items without dropping below 60fps, verified via React DevTools Profiler Flamegraph.
  - Scroll through 1,000+ items: no DOM-overflow or layout breakage across category boundaries.
  - Enabled/total count badge correctly reflects the active safety corridor (Safe/Unsafe) without needing a manual refresh.
  - Bulk enable/disable of 100 mods batches all objectlist count updates into a single render tick.
  - Drag-and-drop folder move completes in ≤ 500ms (disk write + cache invalidate) — including archive extraction if needed.

---

## 2. User Experience & Functionality

### User Stories

#### US-07.1: Virtualized Rendering

As a performance-conscious user, I want the objectlist to stay responsive with thousands of objects, so that the app never freezes during navigation.

| ID        | Type        | Criteria                                                                                                                                                                             |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-07.1.1 | ✅ Positive | Given a list of ≥ 1,000 objects, when the objectlist renders, then only the DOM nodes for visually visible rows are created (virtualized) — maximum 30 DOM nodes at any time         |
| AC-07.1.2 | ✅ Positive | Given rapid scroll through ≥ 1,000 items, then scroll is ≥ 60fps — no blank white flickers lasting > 1 frame (16ms)                                                                  |
| AC-07.1.3 | ❌ Negative | Given resource-constrained hardware (< 4GB RAM), when scrolling violently, the list drops frames visually before freezing the main JS thread — UI remains responsive to clicks       |
| AC-07.1.4 | ⚠️ Edge     | Given a dynamic window resize that changes objectlist height by > 50%, the virtualized list recalculates visible bounds within ≤ 100ms without throwing an out-of-bounds index error |

---

#### US-07.2: Object Selection & Navigation

As a user, I want to click an object in the objectlist to view its mod folders in the center grid, so that I can manage mods for that specific character or entity.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                          |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.2.1 | ✅ Positive | Given a click on an object row, then `selectedObjectId` in Zustand updates within ≤ 16ms and the selected row shows a highlight indicator                                                                                                                                         |
| AC-07.2.2 | ✅ Positive | Given a new `selectedObjectId`, then the center FolderGrid query (`['folders', gameId, objectId]`) invalidates and the new mod list loads within ≤ 200ms from DB                                                                                                                  |
| AC-07.2.3 | ❌ Negative | Given an object that was deleted by a background process while the objectlist was cached, when the user clicks it, then the stale row is removed from the list without an error toast — the action is silently swallowed and the selection remains on the previously valid object |
| AC-07.2.4 | ⚠️ Edge     | Given the user switches the active game, then `selectedObjectId` is immediately cleared to `null` before the new game's object list loads — preventing a cross-game `objectId` in flight                                                                                          |

---

#### US-07.3: Dynamic Enabled Counts

As a user, I want to see real-time enabled/total mod counts on each object row, so that I know at a glance which characters have active mods.

| ID        | Type        | Criteria                                                                                                                                                                                                                                              |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.3.1 | ✅ Positive | Given an object row, then it displays a badge showing `{enabled}/{total}` mod folder counts, derived from the `get_objects` query aggregation                                                                                                         |
| AC-07.3.2 | ✅ Positive | Given the user toggles a mod in the grid (enable/disable), then the objectlist badge for that object updates within ≤ 50ms via optimistic state mutation — before the backend confirms                                                                |
| AC-07.3.3 | ✅ Positive | Given a folder is locked by a parent (`ancestor_disabled_by` is present), then while its internal `is_enabled` flag might be true, it is visually and functionally treated as disabled in the grid; the objectlist count SHOULD eventually reflect "effective" enablement (locked = disabled). |
| AC-07.3.4 | ❌ Negative | Given all mods under an object are disabled (enabled count = 0), then the badge is styled as dim/inactive and the row is not highlighted as "active"                                                                                                  |
| AC-07.3.5 | ⚠️ Edge     | Given a bulk toggle of 100 mods in one action, then all 100 objectlist count increments/decrements are batched into a single React render tick via `unstable_batchedUpdates` or a Zustand immer batch — no frame drops from 100 individual re-renders |

---

#### US-07.4: Zone-Aware Drag-and-Drop

As a user, I want to drag mod folders or archives onto the sidebar to organize them, with specific behavior based on where I drop (Auto Organize, Move to Object, or Create New).

| ID        | Type        | Criteria                                                                                                                                                    |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.4.1 | ✅ Positive | Given a drag over the **Toolbar**, then the "Auto Organize" overlay appears; dropping here triggers a full scan and review modal                            |
| AC-07.4.2 | ✅ Positive | Given a drag over a **Row**, then that row highlights; dropping here moves items into that specific object's folder on disk in ≤ 500ms                      |
| AC-07.4.3 | ✅ Positive | Given a drag over the **Status Bar**, then the "Append as New Object" overlay appears; dropping here opens the `CreateObjectModal` with paths pre-populated |
| AC-07.4.4 | ✅ Positive | Given an archive (.zip, .7z, .rar) is dropped, then an interactive extraction modal allows the user to extract or skip before the organize action continues |
| AC-07.4.5 | ❌ Negative | Given the target object already contains a mod with the same name, then a conflict resolution dialog is shown (Epic 39) instead of overwriting files        |

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

```rust
// Backend Service: Objects (query.rs)

get_filtered_objects(filter):
  1. Pure DB query; returns objects with naming conflict flags (handled by GC).
  2. Scopes results by active safety corridor (Safe vs Unsafe).

gc_lost_objects(game_id):
  1. Build normalized folder set from physical `mods_path`.
  2. Compare indexed objects against folder set.
  3. Safety Abort: If mods root is missing OR would delete ALL objects, stop immediately.
  4. Cleanup: Delete DB records for objects whose folders are gone.
```

### Integration Points

| Component          | Detail                                                                                 |
| ------------------ | -------------------------------------------------------------------------------------- |
| Data Source        | `commands.getObjectsCmd({ gameId, filter })` → `Vec<ObjectWithCounts>`                 |
| Garbage Collection | Triggered on startup, manual sync, and watcher `Removed` events via `gc_lost_objects`. |
| Safety Guards      | GC aborts if mods dir is unreachable or mass wipe (all objects) detected.              |
| Virtualization     | `@tanstack/react-virtual` — `useVirtualizer({ count, estimateSize: () => 48 })`        |
| DnD                | `dnd-kit` — `useDraggable` (FolderCard) + `useDroppable` (ObjectRow)                   |
| Optimistic Update  | `queryClient.setQueryData(['objects', gameId], updater)` on mod toggle                 |
| Move Command       | `commands.moveMod({ srcPath, targetObjectPath })` — atomic rename on disk              |
| Batch Render       | React 18 automatic batching — all count updates within one async event are batched     |

### Security & Privacy

- **Read-only objectlist** — the object list itself displays data but does not mutate any filesystem path or DB record; all mutations go through specific IPC commands (`move_mod`) with validated paths.
- **Safe Mode filter**: ObjectList ALWAYS shows all objects regardless of safe mode (to prevent the navigation pane from disappearing). Instead of removing objects from the list, counts are purely based on Corridors (Safe Mode ONLY counts mods in the safe corridor, Unsafe Mode ONLY counts mods in the unsafe corridor). Items with no mods in the current corridor show `0/0`.

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap), Epic 02 (Game Management — `activeGameId`), Epic 05 (Workspace Layout — panel shell), Epic 06 (ObjectList — rendering container), Epic 09 (Object Schema — category grouping).
- **Blocks**: Epic 12 (Folder Grid — listens to `selectedObjectId`), Epic 15 (Explorer Interactions — DnD source), Epic 40 (Metadata Actions — object pinning).
