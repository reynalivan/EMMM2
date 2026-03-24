# Corridor Switch: Fix Collection Restore Logic

## Context

The corridor switch `restore_target` step was blindly re-enabling all SYSTEM-disabled mods instead of re-applying the target corridor's last active named collection. User clarified the intended behavior:

- If target corridor had a saved collection → re-apply that collection
- If no saved collection (unsaved state) → fall back to restoring SYSTEM-disabled mods

## Changes

- **`switch_pipeline.rs` → `restore_target`**: Rewrote to check `corridor.active_collection_id` first. If present and the collection still exists, delegates to `collection_service::apply_collection` (the same apply_pipeline used by manual collection apply). Falls back to SYSTEM-disabled restore only if no active collection or if re-apply fails.

## Impacted Files

- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)

## Goal

Corridor switch now properly restores the target corridor's last applied collection (if any), preserving the user's intended mod configuration per-corridor.

## Impact

- Corridor switch with active collection now re-applies that collection (enable/disable computed via diff)
- Unsaved corridor state still restores via SYSTEM-disabled fallback
- No breaking changes to frontend or other services
