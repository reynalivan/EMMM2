### Title

Fix Startup Disk Reconcile Unique Constraint Failure

### Context

During startup, Disk Reconcile attempts to insert a mod that it perceives as "new" because the newly generated `folder_path_key` does not match the key stored in the database. When it executes the insertion, it triggers an `(code: 2067) UNIQUE constraint failed: mods.game_id, mods.folder_path` error because the mod is physically at the same case-insensitive path (`COLLATE NOCASE`), but its `folder_path_key` shifted (e.g., from prefix-stripping algorithm changes or renaming nuances).

### Changes

- Upgraded `reconcile_projection_in_tx` fallback logic to locate existing database mods using an exact `to_ascii_lowercase()` path match when the `folder_path_key` lookup misses.
- Refactored `update_mod_identity_tx` and `update_mod_identity` to modify rows specifically by an `id` lookup rather than searching by `folder_path_key`.

### Impacted Files

- `src-tauri/src/services/disk_reconcile/reconcile.rs` (modified)
- `src-tauri/src/repo/mod_repo.rs` (modified)

### Goal

Correctly heal identity drift of existing mods on disk without failing insertions due to unchanged filesystem paths, keeping the disk projection cycle reliable.

### Impact

- Eliminates runtime warnings and duplicate insertion failures during mod reconciliation when older DB structures encounter new parser conventions.
- Robust cross-referencing guarantees we cleanly update stale DB keys dynamically without `UNIQUE` database collisions.
