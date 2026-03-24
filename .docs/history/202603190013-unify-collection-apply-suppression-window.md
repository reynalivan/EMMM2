# Reduce collection apply suppression fragmentation

## Context

- Collection apply flow used fragmented watcher suppression scopes across object state, mod state, and nested mod phases.
- Fragmented scopes increase race surface for watcher events between mutation phases.

## Changes

- Reduced `apply_collection_inner` mutation guard fragmentation by consolidating mod+nested phases under one shared suppression window while preserving existing object-first ordering.
- Added internal no-suppression path for apply state changes:
  - `apply_state_change_without_suppression`
- Added internal no-suppression path for nested apply:
  - `apply_nested_mods_without_suppression`
- Kept existing public APIs (`apply_state_change`, `apply_nested_mods`) suppression-safe for external callers (e.g., undo).
- Moved object-state mutation helper to run without its own internal guard so outer orchestrator owns suppression lifetime.

## Impacted Files

- src-tauri/src/services/collections/apply.rs (modified)

## Goal

- Make collection apply mutation phases more deterministic with fewer suppression boundary transitions.

## Impact

- Reduced suppression-window fragmentation in collection apply path (3 windows -> 2 windows for apply pipeline).
- No command/API/schema changes.

## Notes

- `collections_service` currently reports two failing tests in local run:
  - `collections_apply_then_undo_restores_state`
  - `collections_preview_filters_disabled_unicode_nested_path`
- Privacy fail-fast regressions remain passing (`switch_mode_aborts_when*`).
