# Epic 28: File Watcher (Live Filesystem Monitoring)

## 1. Executive Summary

- **Problem Statement**: Users who organize mods in Windows Explorer while EMMM2 is open expect the app to reflect external changes instantly — without this, the UI shows stale data until a manual refresh, and bulk operations that fire filesystem events trigger unnecessary grid re-renders.
- **Proposed Solution**: A `notify`-crate watcher running as a background Tauri-managed service, watching the active game's `mods_path` with event debouncing (≤ 200ms), filtered through a `SuppressionGuard` that silences internally-generated events, and forwarding only external changes as `fs-changed` events to the frontend via `app_handle.emit`.
- **Success Criteria**:
  - External folder creation appears in the grid within ≤ 500ms of the OS delivering the `Create` event.
  - External folder deletion disappears from the grid within ≤ 500ms.
  - Internal operations (toggle, rename, bulk move) trigger 0 watcher-sourced grid re-fetches while `SuppressionGuard` is active.
  - A panicking suppressed operation drops `SuppressionGuard` via RAII — the watcher resumes normal operation in ≤ 100ms.
  - Watcher switches to the new `mods_path` within ≤ 1s of a game switch (old watcher stopped, new one started).

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
| AC-28.2.1 | ✅ Positive | Given an internal operation (bulk toggle of 100 mods) holds a `SuppressionGuard`, when `notify` delivers rename events for those paths, then the watcher's event handler checks `is_suppressed(path)` and discards those events — 0 `fs-changed` events emitted                |
| AC-28.2.2 | ❌ Negative | Given an internal operation panics while `SuppressionGuard` is held, then the guard's `Drop` implementation removes all suppressed paths from the set — the watcher resumes normal operation within ≤ 100ms                                                                    |
| AC-28.2.3 | ⚠️ Edge     | Given a path is legitimately changed externally at the exact same time as an internal operation suppresses it, then the external event is also suppressed for that path — the frontend will re-fetch on the next React Query `staleTime` expiry (default: 30s) as a safety net |

---

### Non-Goals

- File watcher does not auto-categorize newly discovered folders — only signals "something changed" by invalidating the `['folders']` cache.
- Watcher does not watch for changes inside `.ini` files (content changes) — only directory-level `Create`, `Remove`, `Rename` events.
- No watcher for the OS Recycle Bin - only the active `mods_path`.

---

## 3. Technical Specifications

### Architecture Overview

```
WatcherState (Tauri managed state):
  watcher: Arc<Mutex<Option<RecommendedWatcher>>>
  suppressed_paths: Arc<Mutex<HashSet<PathBuf>>>

init_watcher(game_id) → ():
  1. Stop existing watcher if running
  2. Resolve mods_path for game_id
  3. Create notify::RecommendedWatcher with debounce 200ms:
     event_handler = |event| {
       for path in event.paths:
         if suppressed_paths.contains(&path): continue  // suppress internal ops
         app_handle.emit('fs-changed', FsEvent { kind, path })
     }
  4. watcher.watch(mods_path, RecursiveMode::Recursive)
  5. Store in WatcherState

WatcherSuppression (RAII):
  struct SuppressionGuard { paths: Arc<Mutex<HashSet<PathBuf>>> }
  impl Drop: paths.lock().remove_all(self.guard_paths)

Frontend:
  ExternalChangeHandler.tsx (silent listener, mounted in MainLayout)
    → listen('fs-changed', (event) => {
        queryClient.invalidateQueries(['folders', gameId, subPathForEvent(event.path)])
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
