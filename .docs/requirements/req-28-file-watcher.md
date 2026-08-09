# Epic 28: File Watcher (Live Filesystem Monitoring)

## 1. Executive Summary

- **Problem Statement**: Users who organize mods in Windows Explorer while EMMM is open expect the app to reflect external changes instantly — without this, the UI shows stale data until a manual refresh, and bulk operations that fire filesystem events trigger unnecessary grid re-renders.
- **Proposed Solution**: A `notify-debouncer-full` watcher (over `notify` v7) running as a background Tauri-managed service, watching the active game's `mods_path` recursively with a 500 ms debounce and rename From/To stitching via Windows file IDs. Suppression is two-tier: a blanket ref-counted `SuppressionGuard` for broad operations (scan, archive extraction, import) and path-scoped, identity-keyed registrations (`suppress_paths`, 2 s tail after drop) for toggle/rename/move/trash — so unrelated external events keep flowing during precise mutations. All runtime truth updates go through **Disk Reconcile** — the single writer of `status`/`folder_path` — before typed result payloads reach the frontend; a watcher error/overflow degrades to a force-full reconcile so lost events cannot cause drift.
- **Success Criteria**:
  - [x] External folder creation appears in the grid within ≤ 500ms of the OS delivering the `Create` event.
  - [x] External folder deletion disappears from the grid within ≤ 500ms.
  - [x] Internal operations (toggle, rename, bulk move) trigger 0 watcher-sourced grid re-fetches while the ref-counted suppressor is active.
  - [x] Startup, Mods view entry, refocus, watcher batches, internal mutation completion, and manual repair all converge through Disk Reconcile.
  - [x] Watcher switches to the new `mods_path` within ≤ 1s of a game switch (old watcher stopped, new one started).

---

## 2. User Experience & Functionality

### User Stories

#### US-28.1: Real-Time External Changes

As a user, I want the app to instantly update when I add, delete, or rename a mod folder in Windows Explorer, so that I don't have to manually press "Refresh".

| ID        | Type        | Criteria                                                                                                                                                                                                                  |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-28.1.1 | ✅ Positive | Given the app is open, when I create a new folder in `mods_path/Characters/` via Windows Explorer, then the new folder appears in the `FolderGrid` within ≤ 500ms                                                         |
| AC-28.1.2 | ✅ Positive | Given a folder is deleted externally, then its `FolderCard` disappears from the grid within ≤ 500ms                                                                                                                       |
| AC-28.1.3 | ✅ Positive | Given a folder is renamed externally, then Disk Reconcile updates the DB projection, heals dependent collection paths, and the old card disappears while the new-name card appears within ≤ 500ms                         |
| AC-28.1.5 | ✅ Positive | Given watcher reconciliation reports `folders_changed` or `path_updates`, then ObjectList refreshes immediately because object counts, disabled visuals, and selection paths may have changed                             |
| AC-28.1.4 | ❌ Negative | Given the `mods_path` itself is deleted externally while the watcher is active, then Disk Reconcile returns `status = SourceUnavailable`, performs no DB deletes, and the frontend shows a "Mods folder not found" banner |

---

#### US-28.2: Operation Suppression

As a system, I want the watcher to ignore changes caused by the app's own internal operations, so that bulk actions don't trigger cascading UI re-fetches.

| ID        | Type        | Criteria                                                                                                                                                                                                                                           |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-28.2.1 | ✅ Positive | Given an internal operation holds a suppression (blanket guard for broad ops; path-scoped identity-keyed registration for toggle/rename/move/trash), when a matching filesystem event occurs, the watcher discards it — 0 Disk Reconcile watcher-trigger runs are emitted. Non-matching external events keep flowing during path-scoped suppression. |
| AC-28.2.2 | ✅ Positive | Given nested internal operations hold multiple guards, watcher events remain suppressed until the last guard drops.                                                                                                                                |
| AC-28.2.4 | ❌ Negative | Given a manual compatibility call clears suppression while an inner guard still exists, production mutations still prefer scoped RAII guards; unscoped boolean suppression must not be used as the primary production flow.                        |
| AC-28.2.3 | ⚠️ Edge     | Given the SAME path is legitimately changed externally while an internal operation suppresses it (path-scoped or blanket), that exact event may be skipped — the next window refocus, Mods-entry refresh, or manual Disk Reconcile repairs the projection. External changes to OTHER paths are no longer lost during path-scoped suppression. |

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
  suppressor: Arc<WatcherSuppressor>
    // blanket: guard_depth + manual_depth (ref-counted)
    // path-scoped: Vec<ScopedEntry { identity_key, expires_at }>
    //   suppress_paths([paths]) -> PathSuppressionGuard (RAII, 2s tail on drop)
    //   identity key = DISABLED prefix stripped + case-folded, so ONE entry
    //   covers both spellings of a toggle rename
  watcher: Mutex<Option<Debouncer<RecommendedWatcher, RecommendedCache>>>

start_watcher(game_id) -> ():
  1. Stop existing watcher (Mutex lock + drop), reset manual suppression
  2. Resolve mods_path for game_id
  3. notify_debouncer_full::new_debouncer(500ms):
     callback = |batch| for event in batch:
       if blanket suppressed: drop all
       classify (Renamed via stitched Both, Created, Removed, Modified),
       dropping paths that are irrelevant or path-scoped-suppressed
       tx.send(events)
  4. Spawn background event loop:
     while let Some(batch) = rx.recv() + drain:
       if batch contains watcher Error (overflow -> events LOST):
         result = reconcile_disk_state(ManualRepair, force_full)
       else:
         changed_paths = collect_changed_paths(batch)
         result = reconcile_disk_state_from_watcher_batch(game_id, changed_paths, batch)
       app_handle.emit('disk_reconcile:result', result)

Suppression (RAII):
  SuppressionGuard::new(): blanket depth += 1; Drop: -= 1 (saturating)
  suppress_paths([...]): register identity-keyed entries; Drop: entries live
    another 2s (tail) so async OS events queued during the mutation are
    still swallowed

Quiet reconciles (no 'disk_reconcile:result' event):
  workspace switch, bulk toggle, move-to-object run their scoped reconcile
  via run_internal_disk_reconcile — their command RESULT drives the frontend
  refresh, so emitting too would double-refetch. Import / archive extract /
  trash / meta flows still emit (the event is their only refresh signal).

Disk Reconcile unavailable source:
  if mods_path is missing, unreadable, or not a directory:
    return DiskReconcileResult { status: SourceUnavailable, error_message, ... }
    do not write projection deletes

Frontend:
  RuntimeSyncCoordinator / ExternalChangeHandler.tsx
    -> listen('disk_reconcile:result', (result) => {
        1. Apply `path_updates` + `cleared_selection_paths`
        2. Invalidate objects / folders / thumbnails / collections / dashboard / details
        3. Show batched external-change toast for user-visible object/mod folder changes
      })
```

### Integration Points

| Component         | Detail                                                                                                                                                                                                               |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| notify Crate      | `notify-debouncer-full` over `notify` v7 (ReadDirectoryChangesWatcher on Windows; rename stitching via file IDs)                                                                                                                                                |
| Debounce          | 500 ms debounce window in `notify-debouncer-full`; the event loop drains the channel per callback batch                                                                                                                                                             |
| Suppression       | `Arc<WatcherSuppressor>` shared between watcher and internal file mutations; blanket guards ref-count, path-scoped entries are identity-keyed with a 2 s post-drop tail                                                                                                   |
| Frontend Listener | `listen('disk_reconcile:result', handler)` — registered in the runtime coordinator on mount; applies `path_updates`, then refreshes ObjectList on `objects_changed`, `folders_changed`, or `path_updates.length > 0` |
| Game Switch       | On `set_active_game` → watcher restarts for new `mods_path`; Mods view then runs `reconcileDiskState`                                                                                                                |
| Rename Hints      | The debouncer stitches From/To into one `Renamed { from, to }` event via Windows file IDs; coalesced batches preserve the hints while an older reconcile runs, so collection/path healing is not lost |

### Security & Privacy

- **`mods_path` is the sole watch root** — the watcher never observes paths outside the game's mod directory.
- **Modify events are classified, not ignored** — `.ini` / `info.json` feed dirty-state + keyviewer refresh; thumbnail changes invalidate thumbnail queries so ObjectList row images repaint without manual refresh.
- **Watcher is trigger-only** — canonical runtime truth comes from Disk Reconcile, not from raw watcher events.
- **Watcher never runs Deep Match Scanner** — MasterDB enrichment is explicit user/import workflow only.

---

## 4. Dependencies

- **Blocked by**: Epic 02 (Game Management — `mods_path` per game), Epic 13/14 (Core Mod Ops / Bulk Ops — must be suppressed during internal ops).
- **Blocks**: All file-mutating epics (13, 14, 20, 21, 22) depend on `WatcherSuppression` API being initialized.
