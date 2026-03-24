# Phase 13: Fix Undo Snapshot Duplicate Name Error

## Context

Applying a collection triggered error `Collection name 'Undo Snapshot' already exists in this corridor`. Phase 11's removal of `AND kind = 'named'` from the duplicate check caused all undo snapshots to be rejected after the first one.

## Changes

- Restored kind-aware guard: duplicate name check now only runs for `CollectionKind::Named`
- Added `AND kind = 'named'` back to the SQL query for double safety
- `UndoSnapshot` collections bypass validation entirely (they use fixed system names + unique UUIDs)

## Impacted Files

- `src-tauri/src/repo/collection_repo.rs` (modified)

## Goal

Allow undo snapshots to be created freely during Apply Collection and Switch Corridor operations.

## Impact

- Apply Collection flow is unblocked
- Switch Corridor undo snapshot creation is unblocked
- Dirty state should now work correctly as a cascading fix (apply succeeds → active collection is set → signature comparison works)
