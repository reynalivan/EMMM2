# Epic 22: Trash Safety System

## 1. Executive Summary

- **Problem Statement**: EMMM2 enforces a strict "no data loss" policy — hard-deleting mod folders with `fs::remove_dir_all` is never acceptable; accidental deletes must be recoverable via OS Trash or an in-app Trash Manager.
- **Proposed Solution**: All delete operations use the `trash` Rust crate (`trash::delete()`) to move folders to the OS Recycle Bin, with a fallback custom `.trash/` folder inside `app_data_dir` when cross-drive movement fails. An optional `TrashManagerModal` lists soft-deleted items and allows restore or permanent purge.
- **Success Criteria**:
  - Soft delete (move to Trash) completes in ≤ 500ms for a mod folder ≤ 1GB on SSD.
  - DB record is purged within the same operation (atomic: trash + `DELETE FROM folders`) — never an orphaned DB record.
  - `list_trash` returns results in ≤ 200ms for up to 100 cached soft-deleted entries.
  - `restore_mod` re-materializes the folder at the original path in ≤ 500ms.
  - Hard delete never executes without a second explicit confirmation — 0 accidental permanent deletes.

---

## 2. User Experience & Functionality

### User Stories

#### US-22.1: Soft Delete (Move to Trash)

As a user, I want clicking "Delete" to move the mod to a safe place, so that I can recover it if I made a mistake.

| ID        | Type        | Criteria                                                                                                                                                                                                                  |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-22.1.1 | ✅ Positive | Given a mod folder, when I delete via context menu and confirm, then the folder is moved to OS Recycle Bin via `trash::delete()` and the `folders` DB record is purged in ≤ 500ms                                         |
| AC-22.1.2 | ✅ Positive | Given the delete succeeds, then: (1) the `FolderCard` disappears from the grid via optimistic splice, and (2) the parent Object's `total_count` and `enabled_count` decrement in the objectlist                              |
| AC-22.1.3 | ❌ Negative | Given OS Trash is unavailable (full, cross-drive restriction), then `trash::delete()` fails; the app falls back to moving the folder to `{app_data_dir}/.trash/{uuid}/` with metadata JSON; the DB record is still purged |
| AC-22.1.4 | ⚠️ Edge     | Given a mod folder that is in use by the game engine (file lock), when delete is attempted, then the OS returns a lock error; the app shows "Cannot delete — game may be running"; no partial move occurs                 |

---

#### US-22.2: Trash Manager (In-App Recovery)

As a user, I want to view and manage soft-deleted mods from the app, so that I can restore or permanently purge items without using the OS Recycle Bin.

| ID        | Type        | Criteria                                                                                                                                                                                            |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-22.2.1 | ✅ Positive | Given the Trash Manager Modal, when opened, then it lists all app-managed soft-deleted mods (those moved to `{app_data_dir}/.trash/`) with: original path, mod name, deleted-at timestamp, and size |
| AC-22.2.2 | ✅ Positive | Given clicking "Restore" on a trash item, then the folder is moved back to its original path in ≤ 500ms, the `folders` DB record is re-inserted, and the grid refreshes                             |
| AC-22.2.3 | ✅ Positive | Given clicking "Empty Trash", then a confirmation dialog appears — on confirm, all folders in `{app_data_dir}/.trash/` are `fs::remove_dir_all`'d and the trash metadata DB table is cleared        |
| AC-22.2.4 | ❌ Negative | Given clicking "Restore" but the original path now contains a different folder (conflict), then a `ConflictResolveDialog` appears offering: Restore to a new path, or Skip                          |
| AC-22.2.5 | ⚠️ Edge     | Given the app's `.trash/` folder exceeds 5GB of total size, then the Trash Manager displays a warning banner "Trash is large — consider emptying it" — no automatic purge                           |

---

### Non-Goals

- No browsing of OS Recycle Bin contents — only the app-managed `{app_data_dir}/.trash/` is tracked.
- No "Restore" for items sent to the OS Recycle Bin (not trackable after the fact without OS-specific APIs per platform).
- No undo stack — Trash IS the undo mechanism for deletes.
- No file-level restore (only whole mod folder restore).

---

## 3. Technical Specifications

### Architecture Overview

```
delete_mod(game_id, folder_path) → Result<(), CommandError>:
  1. Validate folder_path (canonicalize + starts_with mods_path)
  2. Acquire OperationLock(game_id) + WatcherSuppression(folder_path)
  3. trash::delete(folder_path)
     → on failure (cross-drive or Trash unavailable):
       uuid = generate()
       fs::rename(folder_path, {app_data_dir}/.trash/{uuid}/)
       INSERT INTO trash_items(uuid, original_path, deleted_at, game_id)
  4. DELETE FROM folders WHERE folder_path = ?  [txn]
  5. Return Ok(())

restore_mod(game_id, trash_uuid) → Result<(), CommandError>:
  1. SELECT original_path FROM trash_items WHERE uuid = ?
  2. Check original_path.exists() → Conflict if true
  3. fs::rename({app_data_dir}/.trash/{uuid}/, original_path)
  4. DELETE FROM trash_items WHERE uuid = ?
  5. INSERT INTO folders (...) [re-registration]
```

### Integration Points

| Component     | Detail                                                                                           |
| ------------- | ------------------------------------------------------------------------------------------------ |
| OS Trash      | `trash` crate `trash::delete(path)` — Recycle Bin on Windows                                     |
| Fallback      | `{app_data_dir}/.trash/{uuid}/` — SQLite `trash_items` table tracks metadata                     |
| Optimistic UI | `queryClient.setQueryData(['folders', gameId, subPath], prev => prev.filter(...))` on `onMutate` |
| DB Purge      | `DELETE FROM folders WHERE folder_path = ?` — in same `sqlx` transaction as trash move           |
| Trash Manager | `TrashManagerModal.tsx` → `list_trash`, `restore_mod`, `empty_trash` commands                    |

### Security & Privacy

- **`folder_path` validated** before any operation — `canonicalize()` + `starts_with(mods_path)` — no arbitrary path deletion possible.
- **Hard delete (`fs::remove_dir_all`) only executes after explicit "Empty Trash" secondary confirmation** — never called during a normal delete workflow.
- **Restore collision guard** runs inside `OperationLock` — check and move are atomic.

---

## 4. Dependencies

- **Blocked by**: Epic 13 (Core Mod Ops — delete action entry point), Epic 14 (Bulk Operations — `bulk_delete_mods`), Epic 15 (Context Menu — delete trigger), Epic 28 (File Watcher — WatcherSuppression).
- **Blocks**: Nothing — leaf safety system.
