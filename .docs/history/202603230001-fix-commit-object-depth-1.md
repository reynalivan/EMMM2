# Fix Commit Object Depth-1 and SQLx Check

### Context

Ensuring type-safety with new `init.sql` schema and restoring strict depth-1 object boundary in `commit.rs` as requested by user.

### Changes

- Restored `STRICT DEPTH-1 OBJECT BOUNDARY` logic in `src-tauri/src/services/scanner/sync/commit.rs` by enforcing the physical `obj_name` over fuzzy `matched_name`.
- Changed `folder_path_key` visibility to `pub` in `src-tauri/src/services/path_key.rs` to fix `debug_keys` compile error.

### Impacted Files

- `src-tauri/src/services/scanner/sync/commit.rs` (modified)
- `src-tauri/src/services/path_key.rs` (modified)

### Goal

- Type Safe DB queries and strict mapping of object physical directory structure. Verified split tables `collection_mods` and `collection_objects` are active.

### Impact

- Objects are protected from improper aggregation during deep matching.
