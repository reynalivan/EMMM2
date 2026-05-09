# Epic 07: Object List

## 1. Executive Summary

- **Problem Statement**: The object list needs to handle thousands of game objects (characters, weapons, UI elements) without freezing the UI — and must keep mod counts (total/enabled) accurate in real-time as the filesystem changes.
- **Proposed Solution**: A virtualized object list powered by `@tanstack/react-virtual`, backed only by the SQLite runtime projection maintained by Disk Reconcile for path/status/count truth, with zone-aware drag-and-drop support (Toolbar: Auto, Row: Move, Status: Append), and a forced selection reset on game switch.
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
| AC-07.2.1 | ✅ Positive | Given a click on an object row, then `selectedObjectFolderPath` in Zustand updates within ≤ 16ms and the selected row shows a highlight indicator                                                                                                                                 |
| AC-07.2.2 | ✅ Positive | Given a new `selectedObjectFolderPath`, then the center FolderGrid refreshes using the current `explorerSubPath` / `mod-folders` query model and the new mod list loads within ≤ 200ms from DB                                                                                    |
| AC-07.2.3 | ❌ Negative | Given an object that was deleted by a background process while the objectlist was cached, when the user clicks it, then the stale row is removed from the list without an error toast — the action is silently swallowed and the selection remains on the previously valid object |
| AC-07.2.4 | ⚠️ Edge     | Given the user switches the active game, then `selectedObjectFolderPath` is immediately cleared to `null` before the new game's object list loads — preventing a cross-game object path in flight                                                                                 |

---

#### US-07.3: Dynamic Enabled Counts

As a user, I want to see real-time enabled/total mod counts on each object row, so that I know at a glance which characters have active mods.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                       |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.3.1 | ✅ Positive | Given an object row, then it displays a muted chip showing `{enabled}/{total}` terminal mod counts, derived from the DB projection refreshed by Disk Reconcile                                                                                                                                 |
| AC-07.3.2 | ✅ Positive | Given the user toggles a deterministic terminal mod in the grid (enable/disable), then the objectlist badge for that object updates optimistically before backend settle; ambiguous flows may fall back to immediate active refetch                                                            |
| AC-07.3.3 | ✅ Positive | Given a folder is locked by a parent (`ancestor_disabled_by` is present), then while its internal `is_enabled` flag might be true, it is visually and functionally treated as disabled in the grid; the objectlist count SHOULD eventually reflect "effective" enablement (locked = disabled). |
| AC-07.3.4 | ❌ Negative | Given all mods under an object are disabled (enabled count = 0), then the chip is styled as dim/inactive and the row gains an explicit inactive visual state                                                                                                                                   |
| AC-07.3.5 | ⚠️ Edge     | Given a bulk object enable/disable of 100 items in one action, then optimistic object-root state changes are applied first and the objectlist performs a single active refresh at the end instead of one refresh per item                                                                      |

---

#### US-07.4: Zone-Aware Drag-and-Drop

As a user, I want to drag mod folders or archives onto the sidebar to organize them, with specific behavior based on where I drop (Auto Organize, Move to Object, or Create New).

| ID        | Type        | Criteria                                                                                                                                                    |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-07.4.1 | ✅ Positive | Given a drag over the **Toolbar**, then the "Auto Organize" overlay appears; dropping here triggers the official Deep Match Scanner preview + review modal  |
| AC-07.4.2 | ✅ Positive | Given a drag over a **Row**, then that row highlights; dropping here moves items into that specific object's folder on disk in ≤ 500ms                      |
| AC-07.4.3 | ✅ Positive | Given a drag over the **Status Bar**, then the "Append as New Object" overlay appears; dropping here opens the `CreateObjectModal` with paths pre-populated |
| AC-07.4.4 | ✅ Positive | Given an archive (.zip, .7z, .rar) is dropped, then an interactive extraction modal allows the user to extract or skip before the organize action continues |
| AC-07.4.5 | ❌ Negative | Given the target object already contains a mod with the same name, then a conflict resolution dialog is shown (Epic 39) instead of overwriting files        |

---

### Non-Goals

- No inline renaming of objects directly in the objectlist.
- No custom avatar upload UI in the objectlist rows; rows may display the discovered/current thumbnail image when available.
- No multi-object selection or drag-group operations.
- No user-created "custom objects" or tags beyond what the `GameSchema` defines.
- No network-fetching of object metadata; all displayed runtime existence/path/count data comes from the local projection tables.
- ObjectList does not run MasterDB/schema matching as a side effect of passive refresh.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: Objects (query.rs)

get_filtered_objects(filter):
  1. Pure DB query over the Disk Reconcile projection.
  2. Scopes results by active safety corridor (Safe vs Unsafe).
  3. Returns physical identity as primary fields (`name`, `folder_path`).
  4. Returns Deep Match Scanner relation fields (`matched_entry_key`, `matched_alias_name`) as secondary metadata only.
```

### Integration Points

| Component          | Detail                                                                                                                                                                          |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Data Source        | `commands.getWorkspaceViewModel({ input })` → `WorkspaceViewModel.objects` from the Disk Reconcile projection                                                                   |
| Disk Reconcile     | `commands.reconcileDiskState(...)` keeps ObjectList projection aligned with add/remove/rename/move/status changes on disk                                                       |
| Garbage Collection | Internal cleanup step owned by startup reconciliation and Disk Reconcile; not a primary ObjectList refresh path                                                                 |
| Safety Guards      | GC aborts if mods dir is unreachable or mass wipe (all objects) detected                                                                                                        |
| Virtualization     | `@tanstack/react-virtual` — `useVirtualizer({ count, estimateSize: () => 48 })`                                                                                                 |
| DnD                | `dnd-kit` — `useDraggable` (FolderCard) + `useDroppable` (ObjectRow)                                                                                                            |
| Optimistic Update  | Shared object-query patch helpers update row name/image/pin/object-disabled state immediately; terminal mod toggles may optimistically patch `enabled_count` when deterministic |
| Move Command       | `commands.moveMod({ srcPath, targetObjectPath })` — atomic rename on disk                                                                                                       |
| Batch Render       | React 18 automatic batching — all count updates within one async event are batched                                                                                              |

### Security & Privacy

- **Read-only objectlist** — the object list itself displays data but does not mutate any filesystem path or DB record; all mutations go through specific IPC commands (`move_mod`) with validated paths.
- **Safe Mode filter**: ObjectList ALWAYS shows all objects regardless of safe mode (to prevent the navigation pane from disappearing). Instead of removing objects from the list, counts are purely based on Corridors (Safe Mode ONLY counts mods in the safe corridor, Unsafe Mode ONLY counts mods in the unsafe corridor). Items with no terminal mods in the current corridor do not render a count chip.
- **Domain Boundary**: ObjectList runtime freshness comes from Disk Reconcile. It must not trigger or depend on Deep Match Scanner unless the user explicitly starts a scan/import flow.
- **Runtime Default**: Objects discovered from disk remain in the runtime `Other` bucket until the user explicitly runs Deep Match Scanner.

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap), Epic 02 (Game Management — `activeGameId`), Epic 05 (Workspace Layout — panel shell), Epic 06 (ObjectList — rendering container), Epic 09 (Object Schema — category grouping).
- **Blocks**: Epic 12 (Folder Grid — listens to `selectedObjectFolderPath`), Epic 15 (Explorer Interactions — DnD source), Epic 40 (Metadata Actions — object pinning).
