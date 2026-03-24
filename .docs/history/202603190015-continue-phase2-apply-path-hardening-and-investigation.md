# Continue phase 2 apply-path hardening and investigation

## Context

- Phase 2 continuation focused on deterministic apply behavior and ongoing collections regression investigation.
- `collections_service` had persistent failures in apply/preview regressions during this pass.

## Changes

- Hardened `apply_collection_inner` with explicit phase-context error mapping for target/snapshot/object/mod/nested phases.
- Added path normalization in `is_foldergrid_level_mod_path` and regression-oriented unit coverage for mixed separators (Windows).
- Extended target-id lookup in apply target resolution to use existing preview path variants before DB key lookup.

## Impacted Files

- src-tauri/src/services/collections/apply.rs (modified)
- src-tauri/src/services/collections/root_resolution.rs (modified)

## Goal

- Continue Phase 2 determinism improvements while narrowing root causes of remaining collections regressions.

## Impact

- Improved diagnosability of apply failures by phase.
- Added separator-stability checks for path classification.
- Remaining collection regressions are still open and require deeper domain-specific fix.

## Notes

- Still failing in local run:
  - collections_apply_then_undo_restores_state
  - collections_preview_filters_disabled_unicode_nested_path
- Privacy fail-fast regressions remain passing.
