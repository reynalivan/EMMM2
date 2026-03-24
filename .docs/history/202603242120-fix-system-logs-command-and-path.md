# Fix System Logs Command and Path

## Context

The "View UI Logs" feature in Settings was broken because the frontend was calling a non-existent `get_logs` command (it was named `get_log_lines` in the backend), and the log file path was incorrectly hardcoded to a location that didn't exist in Tauri v2.

## Changes

- **Backend**: Renamed `get_log_lines` to `get_logs` in `app_cmds.rs`.
- **Backend**: Updated `get_logs` to handle optional `limit` and `count` parameters.
- **Backend**: Changed log path resolution from `app_data_dir/logs` to the official `app_log_dir()` in both `get_logs` and `open_log_folder`.
- **Backend**: Updated command registration in `lib.rs` and `specta_tests`.
- **Permissions**: Added `get_logs` to the allowed commands list in `app-commands.toml`.

## Impacted Files

- `src-tauri/src/commands/app/app_cmds.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)

## Goal

Restore functional access to application logs from the Settings UI.

## Impact

- Users can now view recent system logs directly in the app.
- "Command not found" and "Log file not found" errors are resolved.
- Maintain better compatibility with Tauri v2 filesystem standards.
