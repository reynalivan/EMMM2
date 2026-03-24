# System Stabilization: Hotkey & Watcher Registry Fixes

## Context

Adding new features to the Storage Optimizer revealed several structural regressions in existing core systems (Hotkey Manager and Filesystem Watcher) due to recent domain model updates. These regressions blocked the main application build and caused test failures in unrelated modules.

## Changes

### Hotkey System Recovery

- **Missing Actions**: Restored `NextVariantFolder` and `PrevVariantFolder` variants to `HotkeyAction`.
- **Exhaustive Matching**: Updated `action_label` in `hotkey_cmds.rs` and `dispatch_action` in `manager.rs` to handle the new variants, preventing non-exhaustive match errors.
- **Test Alignment**: Updated `manager_tests.rs` to match the new `dispatch_action` signature (added `suppressor` argument).

### Watcher & Overlay Lifecycle

- **Trigger Alignment**: Fixed `trigger_overlay_refresh` calls in `lifecycle.rs` and `cmds.rs` to include the missing `game_id` and parity arguments.
- **Import Stabilization**: Restored missing `Arc` and `AtomicBool` imports in the watcher service.

### API & Command Hygiene

- **Signature Parity**: Synchronized backend command arguments with frontend `invoke()` calls in the hotkey settings and mod operation flows.

## Impacted Files

- `src-tauri/src/services/hotkeys/mod.rs` (modified)
- `src-tauri/src/services/hotkeys/actions.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)
- `src-tauri/src/services/hotkeys/tests/manager_tests.rs` (modified)
- `src-tauri/src/commands/app/hotkey_cmds.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)

## Goal

Restore the application to a clean-building, stable state while preserving all new feature functionality.

## Impact

- **Build**: 0 compilation errors across Rust and TypeScript.
- **Stability**: Watcher events correctly propagate to the overlay refresh pipeline.
- **Reliability**: Hotkey dispatching remains exhaustive and type-safe.
