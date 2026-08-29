---
trigger: model_decision
description: Data & Filesystem Rules - SQLx, migrations, watcher suppression, and locks.
---

- Watcher: Filesystem mutations own `SuppressionGuard` and terminal reconcile in Rust. Frontend blanket suppression is not allowed.
- Lock: Acquire OperationLock BEFORE mutating mods_path.
- Sync: FS is truth. Runtime refresh uses Disk Reconcile; explicit canonical assignment uses Deep Match Scanner.
- SQLx: query! macros only. Parameter bind (?). Atomic transactions for multi-table.
- Disk Reconcile vs Deep Match Scanner:
  - USE `reconcile_disk_state_cmd` for runtime FS → DB projection updates.
  - USE `deepmatch_preview_cmd` and `deepmatch_scanner_cmd` only for explicit matching/import flows.
  - NEVER replace Disk Reconcile with Deep Match Scanner for watcher or focus-driven sync.
- DB Indexing: FKs/Filter-columns (game_id, is_safe). No SELECT \*.
- Trash: Use the native OS recycle bin through the `trash` crate; do not add an app-managed legacy Trash store.
- Smart Extract: Discover shallowest .ini; split multi-pack archives.
- Lazy Sync: Scan checks folder mtime vs cache.
- Atomic: Multi-file ops MUST have transactional rollback.
- INI: Line-based parser mandated for 3DMigoto.
- ModPackRoot: Folder with root .ini AND ≥2 assets (.dds, .ib, .vb).
- VariantContainer: Folder with root filename= refs OR ≥5 sub-mod babies.
- ContainerFolder: Navigation-only folder.
- InternalAssets: Referent sub-folders; ALWAYS hidden from UI.
