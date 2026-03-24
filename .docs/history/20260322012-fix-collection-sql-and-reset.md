# Fix Collection SQL and App Reset Errors

## Context

The user encountered three major issues:

1. Database reset failed with `no such table: collection_signatures`.
2. Saving collections failed with a SQL alias typo (`o.actual_name` / `o.folder_path_key`).
3. The preview panel incorrectly interpreted Object sub-folders and scanned up to 6 levels deep, mingling the FolderGrid filesystem concepts with the DB Objects instead of recognizing immediate Depth 1 nested folders.

## Changes

- **SQL Typo Fixed:** Modified `create_collection` and `preview_apply` in `collection_service.rs` to correct the invalid `o.folder_path_key` reference and restored the proper `UNION ALL` implementation to keep track of active nested sub-mods alongside objects.
- **Depth-1 Object Normalization:** Completely rewrote `get_collection_preview` to stop generic recursive 6-level scanning and instead correctly assess `Mods/[Object Name]/` strictly for Depth 1 sub-folders, representing exact physical nested mods and appropriately tagging them as enabled/disabled based on `.disabled` prefixes.
- **Resilient Reset Protocol:** Wrapped `reset_all_data` in `settings_repo.rs` with safe PRAGMA foreign key toggles and direct `DROP VIEW/TABLE IF EXISTS collection_signatures` guards prior to dropping core tables, avoiding downstream crashes caused by orphaned table references.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/database/settings_repo.rs` (modified)

## Goal

To guarantee collection saves correctly target exact SQL schemas without erroring out, enforce logical 1:1 parity between Object entries and their physical Depth 1 sub-mods in the UI, and harden the database reset operation.

## Impact

- Collection Preview now accurately renders the immediate sub-mods tied directly to the Object container instead of recursive mess.
- Resolves saving collections throwing backend SQL errors.
- Resolves DB reset throwing error code 1 due to `collection_signatures` constraint.
