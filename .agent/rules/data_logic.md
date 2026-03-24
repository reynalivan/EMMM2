---
trigger: model_decision
description: Data & Filesystem Rules - SQLx, migrations, watcher suppression, and locks.
---

- Watcher: Use SuppressionGuard (Rust) / `commands.setWatcherSuppression` (TS, from `bindings.ts`) for mutations.
- Lock: Acquire OperationLock BEFORE mutating mods_path.
- Sync: FS is truth. Change -> Immediate DB Update.
- SQLx: query! macros only. Parameter bind (?). Atomic transactions for multi-table.
- Indexing vs Auto-Organize: USE `quickImport` (sends empty MasterDB) to ONLY index disk folders to DB silently. DO NOT use `scanPreview` or `syncDatabase` for silent indexing, as they trigger Auto-Organize (Deep Matcher mapping).
- DB Indexing: FKs/Filter-columns (game_id, is_safe). No SELECT \*.
- Trash: Use `trash` crate; fallback to `app_data/.trash/{uuid}` for cross-drive.
- Smart Extract: Discover shallowest .ini; split multi-pack archives.
- Lazy Sync: Scan checks folder mtime vs cache.
- Atomic: Multi-file ops MUST have transactional rollback.
- INI: Line-based parser mandated for 3DMigoto.
- ModPackRoot: Folder with root .ini AND ≥2 assets (.dds, .ib, .vb).
- VariantContainer: Folder with root filename= refs OR ≥5 sub-mod babies.
- ContainerFolder: Navigation-only folder.
- InternalAssets: Referent sub-folders; ALWAYS hidden from UI.
