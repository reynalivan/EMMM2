# Phase 17: Fix UNION ALL Query Errors and Audit Collection Mechanisms

## Context

During testing of Phase 16, saving a collection or applying an undo snapshot caused an SQL compilation error indicating `no such column: o.actual_name`. Additionally, the user requested an audit of the `save`, `switch`, and `delete` collection mechanisms to ensure no edge-cases.

## Changes

- **Fix `o.actual_name`:** Correctly mapped the schema by using `o.name as actual_name` in the `UNION ALL` queries inside `collection_service::create_collection`, `collection_service::preview_apply`, and `snapshot_state::snapshot`. The `objects` table uses `name`, not `actual_name`.
- **Audit `switch_pipeline.rs`:** Discovered that exactly like Phase 16, the `snapshot_leaving` function was still using the old query that solely captured `mods` when switching Privacy Corridors. Replaced it with the complete `UNION ALL` query to correctly capture `MemberKind::Object` layers in the Undo Switch Snapshot.
- **Audit `delete` / `save`:** Verified the frontend UI and backend services. `delete_collection` correctly updates corridor pointers. There is no structural UI or backend path to destructively overwrite collections (save always creates a new ID), so the data flow remains safe.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/pipeline/steps/snapshot_state.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)

## Goal

Eliminate SQL syntax errors during Collection Snapshots, and establish 100% feature parity for reading Object arrays across manually-saved Collections and auto-saved Switch/Undo Snapshots.

## Impact

- Saving a Collection now works cleanly again.
- Switching Safe/Unsafe modes now builds comprehensive Undo Snapshots that correctly list structural Object parents for frontend grouping rendering. No more UI crashes during complex transitions.
