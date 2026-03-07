# Epic 40: Metadata Actions (Pin, Favorite, Move)

## 1. Executive Summary

- **Problem Statement**: Users frequently need three quick organizational actions — pinning high-priority Characters/Objects to the top of the objectlist, favoriting individual mod cards for quick filtering, and moving a mis-categorized mod to its correct Object — all of which require both filesystem and DB coordination.
- **Proposed Solution**: Three backend commands: `pin_object` (DB-only, flips `is_pinned` on the Object row), `toggle_favorite` (DB-only, flips `is_favorite` on the folder row), and `move_mod_to_object` (filesystem `fs::rename` + `info.json` update + DB `folder_path` update under `OperationLock` + `SuppressionGuard`).
- **Success Criteria**:
  - `pin_object` and `toggle_favorite` reflect in the UI in ≤ 150ms (DB-only, cache invalidation only).
  - `move_mod_to_object` completes in ≤ 1s for a typical mod folder (SSD) and ≤ 5s for large folders (10GB network drive).
  - Move is atomic: DB `folder_path` is updated in the same SQLite transaction as `info.json` write; filesystem rename is the last step — no orphan rows possible.
  - Collision detection fires before any filesystem operation — zero moved files that overlap with existing targets.
  - 0 orphan DB rows after a failed move — rollback restores the original DB state.

---

## 2. User Experience & Functionality

### User Stories

#### US-40.1: Pin Object to Top of ObjectList

As a user, I want to pin my most-used Objects to the top of the objectlist, so that I don't scroll to find them every session.

| ID        | Type        | Criteria                                                                                                                                                                                                        |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-40.1.1 | ✅ Positive | Given the Object List, when I click the pin icon on any Object, then `UPDATE objects SET is_pinned = true WHERE id = ?` runs; the Object moves to the "Pinned" section at the top of the objectlist within ≤ 150ms |
| AC-40.1.2 | ✅ Positive | Given an Object is already pinned, when I click the pin icon again, then `is_pinned = false` and the Object returns to its default sort position                                                                |
| AC-40.1.3 | ⚠️ Edge     | Given all Objects are pinned, then the "Pinned" section contains all items and the main list is empty — layout doesn't break                                                                                    |

---

#### US-40.2: Favorite a Mod Card

As a user, I want to mark favorite mod cards with a star, so that I can filter the grid to show only favorites.

| ID        | Type        | Criteria                                                                                                                                                                                  |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-40.2.1 | ✅ Positive | Given the Folder Card context menu, when I click "Favorite", then `UPDATE folders SET is_favorite = true WHERE folder_path = ?` runs; a gold star icon appears on the card within ≤ 150ms |
| AC-40.2.2 | ✅ Positive | Given the grid filter is set to "Favorites Only", then only `is_favorite = true` cards are shown                                                                                          |
| AC-40.2.3 | ⚠️ Edge     | Given I favorite a mod while viewing filtered results, the card stays in view (favorite toggled); un-favoriting it removes it from the filtered view                                      |

---

#### US-40.3: Move Mod to Different Object

As a user, I want to move a mis-categorized mod to its correct Object without using Windows Explorer, so that my library stays consistent.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                        |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-40.3.1 | ✅ Positive | Given a FolderCard context menu "Move to...", when I select a different Object, then the folder is physically moved to `mods_path/{category}/{new_object_name}/{folder_basename}`; `info.json` is updated with the new Object reference; DB `folder_path` is updated atomically |
| AC-40.3.2 | ✅ Positive | Given the move succeeds, then the mod disappears from the old Object's grid and appears under the new Object within ≤ 200ms (via `queryClient.invalidateQueries`)                                                                                                               |
| AC-40.3.3 | ❌ Negative | Given the target path already exists a folder with the same name, then the move is aborted before any filesystem operation; toast shows "Move failed: a folder named '{name}' already exists in '{target_object}'"                                                              |
| AC-40.3.4 | ❌ Negative | Given the `fs::rename` succeeds but the DB update fails (SQLite error), then the DB transaction rolls back — the filesystem is manually reverted (rename back) in the error handler; no orphan DB row is left                                                                   |
| AC-40.3.5 | ⚠️ Edge     | Given the mod is currently enabled when moved, then its enabled state is preserved — the folder name retains its `DISABLED ` prefix state; the new `folder_path` in DB reflects the full new path including any prefix                                                          |

---

### Non-Goals

- No moving entire Objects (all their mods at once) — only individual mod folders.
- No drag-and-drop between Object columns (future enhancement).
- No cross-game moves — moves are scoped to the active `game_id`.

---

## 3. Technical Specifications

### Architecture Overview

```
pin_object(game_id, object_id, is_pinned: bool) → ():
  UPDATE objects SET is_pinned = ? WHERE id = ? AND game_id = ?

toggle_favorite(game_id, folder_path, is_favorite: bool) → ():
  UPDATE folders SET is_favorite = ? WHERE folder_path = ? AND game_id = ?

move_mod_to_object(game_id, folder_path, target_object_id) → ():
  1. Resolve target: SELECT o.name, c.name FROM objects WHERE id = target_object_id
     target_dir = mods_path / category_name / object_name
     target_path = target_dir / folder_basename(folder_path)
  2. if target_path.exists(): return Err(CommandError::PathCollision)
  3. Acquire OperationLock(game_id) + WatcherSuppression([folder_path, target_path])
  4. Update info.json: read → set object field to new object name → write
  5. BEGIN TRANSACTION:
     UPDATE folders SET folder_path = target_path, object_id = target_object_id
       WHERE folder_path = folder_path AND game_id = ?
     COMMIT
  6. fs::create_dir_all(target_dir)
     fs::rename(folder_path, target_path)
     → on error: ROLLBACK + rename back if possible
  7. Release lock + suppression
```

### Integration Points

| Component                                | Detail                                                                           |
| ---------------------------------------- | -------------------------------------------------------------------------------- |
| Pin/Favorite                             | Pure SQLite updates; React Query `['objects']` / `['folders']` invalidation      |
| Move: OperationLock + WatcherSuppression | Same scope as rename/toggle ops                                                  |
| info.json Update                         | Uses `mod_files/metadata.rs` JSON patch — same as Metadata Editor (Epic 17)      |
| Collision Check                          | `target_path.exists()` checked before any operation — feeds Epic 39 if needed    |
| Frontend                                 | Context menu "Move to..." → Object picker modal → `invoke('move_mod_to_object')` |

### Security & Privacy

- **All target paths validated** with `canonicalize()` + `starts_with(mods_path)` before `fs::rename`.
- **`info.json` write is non-destructive** — reads existing JSON, patches only the `object` field, writes back (no overwrite of unrelated keys).
- **DB update precedes `fs::rename`** — if rename fails, DB always has a valid (old) path; if rename succeeds and DB was already updated, the new path is correct.

---

## 4. Dependencies

- **Blocked by**: Epic 09 (Object Schema — target path resolution), Epic 17 (Metadata Editor — `info.json` patch layer), Epic 28 (File Watcher — WatcherSuppression).
- **Blocks**: Nothing — organizational utility actions.
