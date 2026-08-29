## Backend (src-tauri) Layered Architecture Cleanup

### Context
User requested fixing root-level "diseases" in the `src-tauri` backend without abandoning the existing Layered Architecture.

### Changes
- Eliminated redundant `_service` and `_repo` suffixes on module names (e.g. `game_repo` -> `game`, `workspace_service` -> `workspace`).
- Consolidated all root-level service files (e.g., `workspace_service.rs`, `workspace_switch_service.rs`, `collection_runtime_service.rs`) into feature-specific directories.
- Cleaned up dangling files in `repo/` and relocated `tests/` directories to map perfectly with their owner modules.
- Refactored `use` and symbol imports across the entire Rust codebase.
- Ran `cargo check` and `cargo test` which are now completely green.

### Impacted Files
- `src-tauri/src/repo/*` (moved & renamed files/directories)
- `src-tauri/src/services/*` (moved & renamed files/directories)
- `src-tauri/src/lib.rs` (modified imports)
- `src-tauri/src/commands/*` (modified imports)

### Goal
A cleaner, standard Rust idiomatic Layered Architecture (`src-tauri/src/services/<domain>/mod.rs`) that prevents file bloat at the root directory level.

### Impact
None. Strictly an internal file structure reorganization and module rename. All tests are passing.
