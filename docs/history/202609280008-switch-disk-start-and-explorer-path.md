# Switch disk start and Explorer path recovery

## Context

Switch commands waited for a projection-event listener before reaching the disk mutation. A context-menu reveal could still use the old enabled/disabled folder spelling after a rapid rename.

## Changes

- Start projection listener registration without blocking single or object-bulk switch submission; projection completion still checks the durable backend snapshot.
- Resolve an Explorer target to the existing on-disk enabled/disabled variant before validating configured-root containment. An exact existing path remains preferred.
- Emit stage timings for disk commits taking at least 500 ms so remaining storage delays can be attributed to locking, preparation, journaling, or apply work.
- Add regression tests for listener-registration latency and Explorer target resolution.

## Impacted Files

- `src/features/workspace-runtime/actions/workspaceSwitchOps.ts` (modified)
- `src/features/workspace-runtime/actions/workspaceSwitchOps.test.ts` (modified)
- `src-tauri/src/modules/library/adapters/tauri/mod_core_cmds.rs` (modified)
- `src-tauri/src/modules/library/adapters/tauri/tests/mod_core_cmds_tests.rs` (modified)
- `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs` (modified)
- `docs/history/202609280008-switch-disk-start-and-explorer-path.md` (added)

## Goal

Begin disk switching without waiting for runtime projection setup, and open a mod's current physical folder after its name changes.

## Impact

No database or IPC contract change. Explorer targets still pass configured Mods-root containment and the existing open-path preflight. Slow-switch diagnostics contain timings only, not folder paths.
