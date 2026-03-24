# Harden safe-mode switch fail-fast

## Context

- Corridor switch could continue even when pre-switch snapshot failed, which weakens undo reliability.
- Undo pointer persistence to corridor state could fail silently before mutation phases.

## Changes

- Made privacy `switch_mode` fail fast when `snapshot_current_state` fails.
- Made `switch_mode` fail fast when writing leaving-corridor undo pointer fails.
- Added regression tests to ensure switch aborts before any mod mutation when:
  - snapshot creation fails
  - corridor undo-pointer persistence fails

## Impacted Files

- `src-tauri/src/services/privacy/mod.rs` (modified)
- `src-tauri/src/services/privacy/tests/privacy_service_tests.rs` (modified)

## Goal

- Ensure safe/unsafe corridor switching never starts destructive phases without valid pre-switch recovery state.

## Impact

- Improves deterministic failure behavior and reduces silent partial-risk transitions.
- No schema or command contract changes.

## Notes

- Tests use failure injection (`DROP TABLE collections` and corridor_state triggers) to validate pre-mutation abort behavior.
