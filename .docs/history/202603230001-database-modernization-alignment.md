# 202603230001-database-modernization-alignment

## Title

Database Modernization & Legacy Field Alignment (Ultimate Baseline)

## Context

A recent database refactoring introduced a consolidated `init.sql` schema and changed several field names (`is_safe`, `object_type`, `sub_category`). Existing code in the backend, scanner, and frontend still referenced legacy names (`is_safe_context`, `sub_category`), causing compilation and logic errors.

## Changes

- **Database Schema**: Consolidated all migrations into a single `init.sql` using SQLite `STRICT` mode.
- **Field Renames**:
  - `is_safe_context` → `is_safe` (Backend, Scanner, Tests, Frontend)
  - `category_id` → `object_type` (Backend, Repos, Commands)
  - `sub_category` → `sub_category` (Backend, Repos, Tests)
- **Repo Implementation**: Updated `object_repo.rs` and `mod_repo.rs` to support the new schema while restoring missing functions.
- **Split Collections**: Replaced the legacy `collection_member` table with `collection_mods`, `collection_objects`, and `collection_roots` in both logic and migrations.
- **Scanner/Stable IDs**: Refactored identity stabilization logic in `helpers.rs` to handle split-table collections.

## Impacted Files

- `src-tauri/migrations/20260323000000_init.sql` (added/modified)
- `src-tauri/src/database/object_repo.rs` (modified)
- `src-tauri/src/database/mod_repo.rs` (modified)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/scanner/core/types.rs` (modified)
- `src-tauri/src/services/scanner/sync/helpers.rs` (modified)
- `src-tauri/src/database/unicode_keys.rs` (modified)
- `src-tauri/src/test_utils.rs` (modified)
- `src-tauri/src/services/app/tests/dashboard_tests.rs` (modified)
- `src/types/collection.ts` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- multiple test files in `src-tauri/src/.../tests/`

## Goal

A high-performance, consistent database foundation for Virtual Collections and Safe Mode Corridor, with zero residual legacy naming mismatches.

## Impact

- **Breaking Change**: Older databases using `is_safe_context` will need a fresh `init.sql` run or manual migration (user requested modernization over legacy support).
- **Performance**: BATCHeD operations and split tables significantly improve collection apply/snapshot speeds.
