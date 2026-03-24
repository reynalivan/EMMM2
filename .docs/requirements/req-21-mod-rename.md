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

```rust
// Backend Service: Mod Rename (core_ops.rs)

rename_mod_folder_inner(old_path, new_name):
  1. Acquire OperationLock(game_id).
  2. Map `old_rel` path from DB.
  3. Activate SuppressionGuard for source and target paths.
  4. Perform fs::rename.
  5. Update `info.json` with new `actual_name`.
  6. DB MAINTENANCE:
     - Update parent mod record with `new_rel`.
     - Recursively update child mod paths (`update_child_paths`).
     - If top-level, update parent Object's `folder_path`.
  7. Release lock and guard.
```

### Integration Points

| Component            | Detail                                                                                           |
| -------------------- | ------------------------------------------------------------------------------------------------ |
| Metadata Sync        | `info_json.rs::update_info_json` patches `actual_name` field while preserving other metadata.    |
| Recursive DB Update  | `mod_repo::update_child_paths` handles bulk string replacement for nested mod paths.             |
| Object Linking       | `object_repo::update_object_folder_path` ensures the game object stays pinned to the new folder. |
| Path Traversal Guard | `path_utils::is_path_safe` prevents renaming outside the designated mods directory.              |
| Optimistic Rename    | Frontend updates the card label and path instantly before backend confirmation.                  |
| Conflict Dialog      | `ConflictResolveDialog.tsx` — shown when `CommandError::Conflict` returned                       |

### Security & Privacy

- **New name is validated** on both frontend (inline form) and backend (regex `[^\\/:*?"<>|]`, max 128 chars, not empty) — double-checked.
- **Collision check inside `OperationLock`** — `target_path.exists()` check and `fs::rename` are in the same critical section; no TOCTOU gap.
- **WatcherSuppression** prevents old and new path file events from triggering a grid re-fetch during rename.

---

## 4. Dependencies

- **Blocked by**: Epic 11 (Folder Listing — `folder_path` semantics), Epic 13 (Core Mod Ops — shared OperationLock + WatcherSuppression), Epic 20 (Mod Toggle — same prefix logic).
- **Blocks**: Epic 24 (Conflict Resolution — dialog triggered by rename collision).
