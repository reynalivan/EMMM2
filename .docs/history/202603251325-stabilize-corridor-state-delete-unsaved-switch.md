# Stabilize corridor state after delete, unsaved apply, and mode switch

## Context

Delete/apply/switch flow could leave corridor pointers stale, restore the wrong target state, surface `[object Object]` on switch errors, and expose undo-snapshot behavior that no longer matched the desired collection model.

## Changes

- Removed persistent undo-snapshot usage from apply and switch pipelines.
- Made corridor restore resolve target state as `active collection -> corridor unsaved -> SYSTEM fallback`.
- Prevented switch restore from no-op success when target collection pointer is stale.
- Updated collection deletion to clear stale corridor pointers and promote same-corridor unsaved state when the active collection is deleted.
- Removed collection-page undo button and stopped filtering list rows by undo-target semantics.
- Added structured frontend app-error formatting so switch/apply/delete failures render readable messages instead of `[object Object]`.
- Added targeted backend/frontend regression tests for stale active pointer fallback and readable switch errors.

## Impacted Files

- `src-tauri/src/repo/corridor_repo.rs` (modified)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
- `src-tauri/src/pipeline/steps/update_corridor.rs` (modified)
- `src-tauri/src/pipeline/steps/mod.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/domain/corridor.rs` (modified)
- `src/features/collections/CollectionsPage.tsx` (modified)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.test.tsx` (added)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/types/collection.ts` (modified)
- `src/lib/appError.ts` (added)

## Goal

Collections and corridor switching now behave per corridor without persistent undo snapshots, stale-pointer restores, or unreadable switch errors.

## Impact

- Delete/apply/switch flows now converge on active or unsaved corridor state instead of hidden undo collections.
- Safe/unsafe switch preview and runtime restore now use the same target-resolution logic.
- Undo command wiring remains for compatibility, but UI no longer advertises corridor undo and backend returns no undo state.
- No schema migration was introduced.

## Notes

- Rust unit tests compile, but runtime execution in this environment still crashes with `STATUS_ENTRYPOINT_NOT_FOUND`.
- Existing `src/features/collections/hooks/useCollections.test.ts` still fails on a pre-existing cache expectation unrelated to this fix.
