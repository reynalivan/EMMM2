# Add apply phase error-policy context

## Context

- Collection apply errors could be returned without clear phase attribution, making recovery diagnostics harder.
- Phase 2 requires deterministic and understandable failure behavior.

## Changes

- Added explicit phase-context error mapping in `apply_collection_inner` for:
  - target resolution
  - snapshot
  - object planning
  - object mutation
  - current-state resolution
  - mod mutation
  - nested mutation

## Impacted Files

- src-tauri/src/services/collections/apply.rs (modified)

## Goal

- Improve deterministic failure diagnostics for collection apply pipeline without changing public behavior.

## Impact

- Error messages now identify the failing apply phase more clearly.
- No schema/API/permission changes.

## Notes

- Focused collection test `collections_create_and_list` passes.
- Known regression/baseline failure remains: `collections_apply_then_undo_restores_state`.
