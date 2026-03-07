# Epic 21: Mod Rename Operations

## 1. Executive Summary

- **Problem Statement**: Renaming a mod folder is tricky because the physical folder name encodes state (`DISABLED ` prefix) — a naive rename could strip or double the prefix, and a collision on the target path could silently overwrite another mod folder.
- **Proposed Solution**: A `rename_mod_folder` backend command that: strips and re-applies the `DISABLED ` prefix correctly based on `is_enabled`, validates the new name, checks for path collisions inside the `OperationLock`, updates `info.json` atomically, and returns structured errors with optimistic UI rollback.
- **Success Criteria**:
  - Rename (disk + `info.json` sync) completes in ≤ 500ms for a flat mod folder on SSD.
  - `DISABLED ` prefix is preserved/stripped correctly in 100% of test cases (enabled, disabled, double-prefix).
  - Collision detection prevents overwriting any existing folder 100% of the time (check inside `OperationLock`).
  - Windows invalid-character validation blocks the rename on both frontend and backend before any IPC call.
  - `WatcherSuppression` prevents any rename-triggered re-fetch for the renamed paths.

---

## 2. User Experience & Functionality

### User Stories

#### US-21.1: Rename Mod Securely

As a user, I want to rename my mod folder to better reflect its contents, so that my library stays organized.

| ID        | Type        | Criteria                                                                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-21.1.1 | ✅ Positive | Given an enabled mod `MyMod`, when renamed to `BetterName`, then the physical folder becomes `BetterName` on disk and `info.json` `name` field updates to `BetterName` in ≤ 500ms                                 |
| AC-21.1.2 | ✅ Positive | Given an enabled mod, when renamed, then the resulting folder has no `DISABLED ` prefix — the enabled state is preserved                                                                                          |
| AC-21.1.3 | ✅ Positive | Given a disabled mod `DISABLED OldName`, when renamed to `NewName`, then the resulting folder is `DISABLED NewName` — the disabled prefix is re-applied correctly                                                 |
| AC-21.1.4 | ❌ Negative | Given the new name contains Windows-invalid characters (`\ / : * ? " < > \|`), then the frontend form blocks the submit with inline "Invalid characters in name" — no IPC call is made                            |
| AC-21.1.5 | ❌ Negative | Given the new target path already exists on disk (collision), then `rename_mod_folder` returns `CommandError::Conflict` and the `ConflictResolveDialog` appears — no rename has occurred                          |
| AC-21.1.6 | ⚠️ Edge     | Given a rename where the resulting path would exceed the Windows 260-char limit, then the backend returns `PathTooLongError` with the projected full path length — the rename is blocked before any filesystem op |

---

#### US-21.2: Pre-Delete Check (Content Audit)

As a system, I want to audit a mod folder's file contents before any rename/delete, so that the UI can warn users about rich folders that may take longer or have special handling.

| ID        | Type        | Criteria                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-21.2.1 | ✅ Positive | Given a rename request is initiated, when `pre_delete_check(folderPath)` runs, then it returns `FolderContentInfo { ini_count, image_count, nested_folder_count, total_size_bytes }` in ≤ 200ms |
| AC-21.2.2 | ✅ Positive | Given `ini_count > 0`, then the rename confirmation dialog shows "This folder contains {N} INI files — 3DMigoto references may need updating" as a non-blocking warning                         |
| AC-21.2.3 | ⚠️ Edge     | Given a folder with ≥ 1,000 nested files, then `pre_delete_check` returns in ≤ 500ms (bounded by a timeout — returns an estimate if filesystem is too slow)                                     |

---

### Non-Goals

- No in-app "undo" for rename — if the user needs to reverse, they can rename again.
- Rename does not update any 3DMigoto INI cross-references to the renamed folder — only the folder name and `info.json` `name` field change.
- No regex-based batch rename — single folder rename only in this epic.

---

## 3. Technical Specifications

### Architecture Overview

```
rename_mod_folder(game_id, folder_path, new_base_name) → Result<(), CommandError>:
  1. Validate new_base_name: no forbidden chars, not empty, trimmed, max 128 chars
  2. Acquire OperationLock(game_id).await
  3. Activate WatcherSuppression([folder_path, target_path])
  4. Normalize target name:
      new_name = if is_disabled(folder_path): "DISABLED " + new_base_name else new_base_name
      target_path = parent(folder_path) / new_name
  5. Check target_path.len() <= 260 (Windows MAX_PATH)
  6. Check !target_path.exists() → if exists: return Err(Conflict)
  7. fs::rename(folder_path, target_path)
  8. Update info.json: if (target_path/info.json).exists(): set name = new_base_name
  9. Return Ok(())

pre_delete_check(game_id, folder_path) → FolderContentInfo:
  → bounded walkdir (max 2s) counting .ini files, images, nested dirs, total size
```

### Integration Points

| Component          | Detail                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------- |
| Command            | `mod_core_cmds.rs::rename_mod_folder`                                                       |
| OperationLock      | Shared `Arc<Mutex<()>>` per `game_id`                                                       |
| WatcherSuppression | Both `folder_path` (old name) and `target_path` (new name) suppressed                       |
| Frontend           | `useFolderGridActions.ts::renameFolder` — optimistic rename in cache, rollback on `onError` |
| Conflict Dialog    | `ConflictResolveDialog.tsx` — shown when `CommandError::Conflict` returned                  |

### Security & Privacy

- **New name is validated** on both frontend (inline form) and backend (regex `[^\\/:*?"<>|]`, max 128 chars, not empty) — double-checked.
- **Collision check inside `OperationLock`** — `target_path.exists()` check and `fs::rename` are in the same critical section; no TOCTOU gap.
- **WatcherSuppression** prevents old and new path file events from triggering a grid re-fetch during rename.

---

## 4. Dependencies

- **Blocked by**: Epic 11 (Folder Listing — `folder_path` semantics), Epic 13 (Core Mod Ops — shared OperationLock + WatcherSuppression), Epic 20 (Mod Toggle — same prefix logic).
- **Blocks**: Epic 24 (Conflict Resolution — dialog triggered by rename collision).
