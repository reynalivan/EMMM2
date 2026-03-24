# Unified In-Game Overlay System

## Context

The KeyViewer and Hotkey Status systems were previously fragmented, using an obsolete `ps-t100` rendering method and inconsistent storage paths. This change unifies them into a single, high-performance, toggleable in-game system using the `help.ini` pipeline.

## Changes

- **Rendering**: Switched from custom `ps-t100` shaders to the system `help.ini` pipeline (`ResourceNotification`).
- **Logic**: Unified Toggle (F7) for both Keybinds and Status. Status is shown if no character is detected; character-specific keybinds appear automatically when their shader hash is matched.
- **Storage**: Moved all runtime artifacts (`KeyViewer.ini`, `runtime_status.txt`, `keybinds/*.txt`) to `Mods/.emmm_data/`.
- **Sync**: Added real-time refresh triggers to Hotkey Settings updates, Active Game changes, and Safe Mode toggles.
- **Mockup**: Aligned text formatting with the requested premium design (Arlecchino style).

## Impacted Files

- `src-tauri/src/services/keyviewer/generator.rs` (Modified)
- `src-tauri/src/services/app/post_apply.rs` (Modified)
- `src-tauri/src/services/hotkeys/mod.rs` (Modified)
- `src-tauri/src/services/hotkeys/manager.rs` (Modified)
- `src-tauri/src/commands/app/hotkey_cmds.rs` (Modified)
- `src-tauri/src/commands/app/settings_cmds.rs` (Modified)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (Modified)
- `src/features/settings/tabs/HotkeyTab.tsx` (Modified)
- `src/types/settings.ts` (Modified)

## Goal

A single, clean, and reliable in-game overlay that accurately displays active character hotkeys and systemic status (Safe Mode, Presets) with zero manual configuration.

## Impact

- **Performance**: Overlay logic is now event-driven (via hashes) and leverages the native 3DMigoto formatting pipeline, reducing overhead.
- **Usability**: Simplified settings UI and consistent hotkey behavior.
- **Reliability**: No more stale status banners or mismatched toggle keys.
