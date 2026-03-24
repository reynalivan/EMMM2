# Atomic undo snapshot and corridor_state cleanup

## Context

- Undo flow cleaned snapshot collection and corridor_state in separate operations.
- On metadata failure paths this could leave partial DB state (e.g., snapshot deleted but corridor_state not cleared), reducing determinism.

## Changes

- Added transactional corridor state helper:
  - `corridor_state_repo::upsert_corridor_state_tx`
- Refactored undo finalization to one transaction in `finalize_undo_snapshot_and_state`:
  - clear corridor_state first
  - delete undo snapshot collection
  - commit together
- Updated undo flow to fail fast on finalization errors in both:
  - empty no-op undo path
  - normal undo path after mutations
- Added regression test with failure injection trigger:
  - `test_undo_keeps_snapshot_when_corridor_state_clear_fails`
  - verifies rollback keeps snapshot when corridor_state clear fails.

## Impacted Files

- src-tauri/src/database/corridor_state_repo.rs (modified)
- src-tauri/src/services/collections/undo.rs (modified)
- src-tauri/src/commands/collections/tests/collection_cmds_tests.rs (modified)

## Goal

- Ensure undo metadata cleanup is atomic and deterministic.

## Impact

- Prevents partial cleanup drift in undo failure scenarios.
- Keeps snapshot pointer cleanup behavior consistent under DB trigger failures.
- Existing apply/undo and preview parity regressions remain green.

## Notes

- Verified with:
  - `cargo test --lib test_undo_keeps_snapshot_when_corridor_state_clear_fails -- --nocapture`
  - `cargo test --test collections_service collections_apply_then_undo_restores_state -- --nocapture`
  - `cargo test --lib test_preview_corridor_switch -- --nocapture`
