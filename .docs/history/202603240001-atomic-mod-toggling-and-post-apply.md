# Atomic Mod Toggling and Post Apply Refactor

## Context

The enable-only-this feature (used to switch between conflicting mods) had relative/absolute path mismatches when fetching siblings from the DB, was not atomic in terms of DB updates, and didn't trigger full auto-reloads. The bulk toggle command also bypassed auto-reloads.

## Changes

- Refactored `enable_only_this_service` to resolve absolute paths into relative paths before interacting with the database.
- Used `batch_update_path_and_status` in `enable_only_this_service` to replace multiple individual `update_mod_identity` queries, making the sibling disable and target enable process a single atomic transaction.
- Appended `run_post_apply_tasks` to the end of `enable_only_this` and `bulk_toggle_mods` Tauri commands so that bulk operations reliably trigger the game reload and hotkey reconfiguration.
- Cleaned up unused import lints in `tests` and `archive/extract.rs`.

## Impacted Files

- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)
- `src-tauri/src/commands/scanner/conflict_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_bulk_cmds.rs` (modified)
- `src-tauri/src/commands/mods/tests/mod_bulk_cmds_tests.rs` (modified)
- `src-tauri/src/services/mods/archive/extract.rs` (modified)

## Goal

Safe and transaction-backed mod conflict switching with full app state synchronization and zero unused import compiler warnings.

## Impact

- Fixes path query crashes when trying to disable siblings.
- Resolves DB vs File System consistency edge cases during rapid conflict resolution.
- Ensures the 3DMigoto game engine immediately refreshes assets after mass mod toggling operations via `run_post_apply_tasks`.
