# V1 Dead Code Removal

## Context

Greenfield V2 refactor left orphaned V1 code that referenced deleted modules but was never compiled because nothing called it.

## Changes

- **Deleted** `src-tauri/src/services/privacy/` — entire V1 privacy module (dead code referencing removed `database::collection_repo`, `database::corridor_state_repo`, `services::collections`). Fully replaced by `switch_pipeline.rs` + `corridor_service.rs`.
- **Deleted** `src-tauri/src/commands/collections/tests/collection_cmds_tests.rs` — 783 lines of orphaned V1 tests using removed APIs (`create_collection_service`, `apply_collection_service`, `corridor_state_repo`).
- **Deleted** `src/features/safe-mode/ModeSwitchConfirmModal.test.tsx` — stale test invoking removed `get_corridor_runtime_snapshot` command.

## Impacted Files

- `src-tauri/src/services/privacy/mod.rs` (removed)
- `src-tauri/src/services/privacy/tests/privacy_service_tests.rs` (removed)
- `src-tauri/src/commands/collections/tests/collection_cmds_tests.rs` (removed)
- `src/features/safe-mode/ModeSwitchConfirmModal.test.tsx` (removed)

## Goal

Remove dead V1 code that referenced deleted modules to prevent confusion during future development.

## Impact

- No functional impact — all deleted code was unreachable.
- Reduces codebase by ~1000+ lines of stale V1 code.
