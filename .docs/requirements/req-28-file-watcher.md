# Epic 28: File Watcher (Live Filesystem Monitoring)

## 1. Executive Summary

- **Problem Statement**: Users who organize mods in Windows Explorer while EMMM is open expect the app to reflect external changes instantly — without this, the UI shows stale data until a manual refresh, and bulk operations that fire filesystem events trigger unnecessary grid re-renders.
- **Proposed Solution**: A `notify`-crate watcher running as a background Tauri-managed service, watching the active game's `mods_path` with a poll-based event loop (500ms). It features a global `SuppressionGuard` (AtomicBool) to silence internally-generated events and an event loop that performs atomic DB synchronization before forwarding events to the frontend.
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
| AC-28.1.3 | ✅ Positive | Given a folder is renamed externally, then the old card disappears and the new-name card appears within ≤ 500ms (triggered by `Rename` event → grid invalidation)                                     |
| AC-28.1.4 | ❌ Negative | Given the `mods_path` itself is deleted externally while the watcher is active, then the watcher logs a `warn` and sends a `fs-path-gone` event — the frontend shows a "Mods folder not found" banner |

---

#### US-28.2: Operation Suppression

As a system, I want the watcher to ignore changes caused by the app's own internal operations, so that bulk actions don't trigger cascading UI re-fetches.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-28.2.1 | ✅ Positive | Given an internal operation (bulk toggle) holds a `SuppressionGuard`, when a filesystem event occurs, the watcher checks `suppressor` flag and discards the event — 0 `mod_watch:event` emitted.                                                                               |
| AC-28.2.2 | ✅ Positive | Given an internal operation panics or completes, the RAII `SuppressionGuard` drops and resets the flag to `false` automatically.                                                                                                                                               |
| AC-28.2.3 | ⚠️ Edge     | Given a path is legitimately changed externally at the exact same time as an internal operation suppresses it, then the external event is also suppressed for that path — the frontend will re-fetch on the next React Query `staleTime` expiry (default: 30s) as a safety net |

---

### Non-Goals

- File watcher only tracks relevant extensions: `ini`, `png`, `jpg`, `jpeg`, `webp` and directories (no extension). Other files are ignored.
- Watcher ignores hidden folders starting with `.`.
- Watcher depth check: Frontend filters events only for depth 1 (Object) or depth 2 (Mod) relative to `mods_path`.
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
       sync_watcher_event_batch(db, batch)  [atomic DB mirror sync]
       app_handle.emit('mod_watch:event', batch)

WatcherSuppression (RAII):
  struct SuppressionGuard { suppressor: Arc<AtomicBool> }
  impl Drop: suppressor.store(false)

Frontend:
  ExternalChangeHandler.tsx (silent listener)
    → listen('mod_watch:event', (payload) => {
        if (cooldownActive) return;
        1. Debounce batch (300ms)
        2. Filter isValidModFolder (depth 1 or 2)
        3. Trigger sync_objects_cmd / gc_lost_objects_cmd (if needed)
        4. Invalidate specific queries (objects, folders, category-counts)
        5. Show user-friendly toast if in 'mods' view
      })
```

### Integration Points

| Component         | Detail                                                                                         |
| ----------------- | ---------------------------------------------------------------------------------------------- |
| notify Crate      | `notify::RecommendedWatcher` (ReadDirectoryChangesWatcher on Windows)                          |
| Debounce          | `notify_debouncer_mini` with 200ms delay before event forwarding                               |
| Suppression       | `Arc<Mutex<HashSet<PathBuf>>>` shared between `WatcherService` and all `OperationLock` holders |
| Frontend Listener | `listen('fs-changed', handler)` — registered in `ExternalChangeHandler.tsx` on mount           |
| Game Switch       | On `set_active_game` → `stop_watcher` → `init_watcher(new_game_id)`                            |

### Security & Privacy

- **`mods_path` is the sole watch root** — the watcher never observes paths outside the game's mod directory.
- **Only `Create`, `Remove`, `Rename` event kinds are forwarded** — `Modify` (content changed, not name) is ignored.
- **`is_suppressed` path check uses exact path string matching** — no partial prefix matching that could over-suppress unrelated paths.

---

## 4. Dependencies

- **Blocked by**: Epic 02 (Game Management — `mods_path` per game), Epic 13/14 (Core Mod Ops / Bulk Ops — must be suppressed during internal ops).
- **Blocks**: All file-mutating epics (13, 14, 20, 21, 22) depend on `WatcherSuppression` API being initialized.
