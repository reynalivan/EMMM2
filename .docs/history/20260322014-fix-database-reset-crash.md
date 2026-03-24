# Phase 15: Fix Database Reset Crash on Legacy Tables

## Context

Clicking the "Reset Application Setup" button crashed the backend database reset process with `Failed to reset database: (code: 1) no such table: collection_signatures`. This occurred because the reset function was never updated after Phase 11/12 and still attempted to delete data from 7 legacy V1 tables that were dropped during the greenfield migration.

## Changes

- Removed 7 legacy `DELETE FROM` SQL queries from `settings_repo::reset_all_data`:
  - `collection_signatures`
  - `collection_roots`
  - `corridor_runtime_cache`
  - `collection_object_states`
  - `collection_nested_items`
  - `collection_items`
  - `corridor_state`
- Added new `DELETE FROM` SQL queries for the V2 schema tables correctly ordered by foreign key dependency:
  - `collection_member`
  - `corridor`

## Impacted Files

- `src-tauri/src/database/settings_repo.rs` (modified)

## Goal

Restore application reset functionality by exclusively targeting active V2 tables during data wiping.

## Impact

- The "Reset Application Setup" feature works correctly again without encountering SQLite `no such table` panics.
