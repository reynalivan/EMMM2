# Phase 14: Fix DB UNIQUE Constraint & Apply Preview Dialog

## Context

Applying a collection crashed with `UNIQUE constraint failed: collections.game_id, collections.name_key, collections.is_safe_context` because the hard DB constraint from migration 011 blocked undo snapshot creation. Additionally, the Collections page had no preview dialog before applying.

## Changes

- Created migration `013_drop_unique_name_constraint.sql` to recreate `collections` table without the UNIQUE constraint (SQLite doesn't support ALTER TABLE DROP CONSTRAINT)
- Wired `ApplyCollectionModal` into `CollectionsPage.tsx` via `applyTargetId` local state — clicking Apply now shows a Before/After preview dialog
- Removed unused `useApplyCollection` import from CollectionsPage (modal handles its own mutation)

## Impacted Files

- `src-tauri/migrations/013_drop_unique_name_constraint.sql` (added)
- `src/features/collections/CollectionsPage.tsx` (modified)

## Goal

Allow undo snapshots to coexist in the DB and provide a preview dialog before applying collections.

## Impact

- Apply Collection flow is fully unblocked at both Rust and DB levels
- User sees Before/After preview before any changes are made
- No breaking changes
