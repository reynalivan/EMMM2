# Remove Phantom Tables and Inactive Fallbacks

## Context
Phase 3 of the Legacy & Backward Compatibility Audit. Discovered four database tables and associated logic that were completely dead (never populated or queried successfully).

## Changes
- **Database Cleanup**:
  - Dropped object_sentinel_cache and game_sentinel_settings (dead legacy feature tables).
  - Dropped mod_hash_index (dead feature cache).
  - Dropped download_sessions (dead table causing browser auto-imports to silently fail).
- **Rust Backend Cleanup**:
  - Removed dead query get_session_game_id in import_jobs.rs.
  - Removed dead foreign-key cascade array for mod_hash_index in sync.rs.
  - Cleaned up queue_import_job in queue.rs to no longer rely on phantom session IDs, instead immediately returning the unsupported error that it was functionally returning before.
  - Removed session_id unused argument from the caller in download_service.rs.

## Impacted Files
- src-tauri/migrations/20260323000000_init.sql (modified - merged dead table removals)
- src-tauri/src/repo/browser/import_jobs.rs (modified)
- src-tauri/src/repo/mods/sync.rs (modified)
- src-tauri/src/services/browser/import_service/queue.rs (modified)
- src-tauri/src/services/browser/download_service.rs (modified)

## Goal
A strictly essential database schema and a clean codebase free from phantom\ features and dead fallback paths.

## Impact
- Four legacy tables permanently dropped, preventing developer confusion.
- Removed broken DB lookup fallbacks from the browser queue, simplifying the import service.
- Maintained 100% test pass rate.