# Epic 28: File Watcher (Live Filesystem Monitoring)

## 1. Executive Summary

- **Problem Statement**: Users who organize mods in Windows Explorer while EMMM is open expect the app to reflect external changes instantly — without this, the UI shows stale data until a manual refresh, and bulk operations that fire filesystem events trigger unnecessary grid re-renders.
- **Proposed Solution**: A `notify`-crate watcher running as a background Tauri-managed service, watching the active game's `mods_path` recursively. It uses a global `SuppressionGuard` (AtomicBool) to silence internally-generated events, batches filesystem events, and delegates all runtime truth updates to **Disk Reconcile** (`reconcile_disk_state_cmd`) before emitting typed result payloads to the frontend.
- **Success Criteria**:
  - [x] External folder creation appears in the grid within ≤ 500ms of the OS delivering the `Create` event.
  - [x] External folder deletion disappears from the grid within ≤ 500ms.
  - [x] Internal operations (toggle, rename, bulk move) trigger 0 watcher-sourced grid re-fetches via `suppressor` flag.
  - [x] Automatic GC (Garbage Collection) of lost objects runs whenever a watcher is initialized.
  - [x] Watcher switches to the new `mods_path` within ≤ 1s of a game switch (old watcher stopped, new one started).

---

## 2. User Experience & Functionality

### User Stories

#### US-28.1: Real-Time External Changes

As a user, I want the app to instantly update when I add, delete, or rename a mod folder in Windows Explorer, so that I don't have to manually press "Refresh".

| ID        | Type        | Criteria                                                                                                                                                                                              |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-28.1.1 | ✅ Positive | Given the app is open, when I create a new folder in `mods_path/Characters/` via Windows Explorer, then the new folder appears in the `FolderGrid` within ≤ 500ms                                     |
| AC-28.1.2 | ✅ Positive | Given a folder is deleted externally, then its `FolderCard` disappears from the grid within ≤ 500ms                                                                                                   |
| AC-28.1.3 | ✅ Positive | Given a folder is renamed externally, then Disk Reconcile updates the DB projection, heals dependent collection paths, and the old card disappears while the new-name card appears within ≤ 500ms      |
| AC-28.1.5 | ✅ Positive | Given watcher reconciliation reports `folders_changed` or `path_updates`, then ObjectList refreshes immediately because object counts, disabled visuals, and selection paths may have changed                                                     |
| AC-28.1.4 | ❌ Negative | Given the `mods_path` itself is deleted externally while the watcher is active, then the watcher logs a `warn` and sends a `fs-path-gone` event — the frontend shows a "Mods folder not found" banner |

---

#### US-28.2: Operation Suppression

As a system, I want the watcher to ignore changes caused by the app's own internal operations, so that bulk actions don't trigger cascading UI re-fetches.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-28.2.1 | ✅ Positive | Given an internal operation (bulk toggle) holds a `SuppressionGuard`, when a filesystem event occurs, the watcher checks `suppressor` flag and discards the event — 0 Disk Reconcile watcher-trigger runs are emitted.                                                           |
| AC-28.2.2 | ✅ Positive | Given an internal operation panics or completes, the RAII `SuppressionGuard` drops and resets the flag to `false` automatically.                                                                                                                                               |
| AC-28.2.3 | ⚠️ Edge     | Given a path is legitimately changed externally at the exact same time as an internal operation suppresses it, then that exact event may be skipped — the next window refocus, Mods-entry refresh, or manual Disk Reconcile repairs the projection |

---

### Non-Goals

- File watcher tracks directories plus runtime-relevant files: `.ini`, `info.json`, `png`, `jpg`, `jpeg`, `webp`. Other files are ignored.
- Watcher ignores hidden folders starting with `.`.
- Watcher classifies changes by top-level Object root relative to `mods_path`; Disk Reconcile decides whether the refresh is structural, runtime-file, or thumbnail-only.
- No watcher for the OS Recycle Bin - only the active `mods_path`.

---

## 3. Technical Specifications

### Architecture Overview

```
WatcherState (Tauri managed state):
  suppressor: Arc<AtomicBool>
  watcher: Mutex<Option<RecommendedWatcher>>

init_watcher(game_id) → ():
  1. Stop existing watcher (Mutex lock + drop)
  2. Spawn Auto-GC task (clean orphaned objects)
  3. Resolve mods_path for game_id
  4. Create notify::RecommendedWatcher:
     event_handler = |event| {
       if suppressor.is_active(): return
       translate_event(event) → Vec<ModWatchEvent>
       tx.send(events)
     }
  5. Spawn background event loop:
     while let Ok(batch) = rx.recv_batch():
       changed_paths = collect_changed_paths(batch)
       result = reconcile_disk_state_from_watcher_batch(game_id, changed_paths, batch)
       app_handle.emit('disk_reconcile:result', result)
       app_handle.emit('mod_watch:events_batch', batch)   [informational only]

WatcherSuppression (RAII):
  struct SuppressionGuard { suppressor: Arc<AtomicBool> }
  impl Drop: suppressor.store(false)

Frontend:
  RuntimeSyncCoordinator / ExternalChangeHandler.tsx
    → listen('disk_reconcile:result', (result) => {
        1. Apply `path_updates` + `cleared_selection_paths`
        2. Invalidate objects / folders / thumbnails / collections / dashboard / details
        3. Show batched external-change toast for user-visible object/mod folder changes
      })
```

### Integration Points

| Component         | Detail                                                                                               |
| ----------------- | ---------------------------------------------------------------------------------------------------- |
| notify Crate      | `notify::RecommendedWatcher` (ReadDirectoryChangesWatcher on Windows)                                |
| Debounce          | Watcher batches events before dispatch to Disk Reconcile                                              |
| Suppression       | `Arc<AtomicBool>` shared between watcher and internal file mutations                                  |
| Frontend Listener | `listen('disk_reconcile:result', handler)` — registered in the runtime coordinator on mount; applies `path_updates`, then refreshes ObjectList on `objects_changed`, `folders_changed`, or `path_updates.length > 0` |
| Game Switch       | On `set_active_game` → watcher restarts for new `mods_path`; Mods view then runs `reconcileDiskState` |

### Security & Privacy

- **`mods_path` is the sole watch root** — the watcher never observes paths outside the game's mod directory.
- **Modify events are classified, not ignored** — `.ini` / `info.json` feed dirty-state + keyviewer refresh; thumbnail changes invalidate thumbnail queries so ObjectList row images repaint without manual refresh.
- **Watcher is trigger-only** — canonical runtime truth comes from Disk Reconcile, not from raw watcher events.

---

## 4. Dependencies

- **Blocked by**: Epic 02 (Game Management — `mods_path` per game), Epic 13/14 (Core Mod Ops / Bulk Ops — must be suppressed during internal ops).
- **Blocks**: All file-mutating epics (13, 14, 20, 21, 22) depend on `WatcherSuppression` API being initialized.
