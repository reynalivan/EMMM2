# Unified Overlay Synchronization

## Context

The KeyViewer and Hotkey Status overlays were fragmented and didn't reliably update when settings or mod states changed. This update unifies them into a single, event-driven system using the 3DMigoto `help.ini` pipeline.

## Changes

- **Harvester Fix**: Updated regex to support `0x` prefixes in mod INIs, fixing a major gap where modern mods weren't being detected.
- **Path Resolution**: Fixed a bug where the backend harvester received relative paths, preventing it from reading mod files.
- **Unified Logic**: Consolidated `KeyViewer.ini` and status banner generation into a single post-apply pipeline.
- **Global Sync**: Integrated `trigger_overlay_refresh` into all relevant commands (Hotkey updates, game selection, mod toggling, collection apply/update/delete/switch).
- **Banner Enhancement**: The status banner now automatically fetches and displays the active collection (preset) name from the database.
- **Mockup Alignment**: Reformatted in-game text outputs to match the premium "Arlecchino" design requirements.

## Impacted Files

- `src-tauri/src/services/keyviewer/harvester.rs` (modified regex)
- `src-tauri/src/services/app/post_apply.rs` (fixed paths, added status lookup)
- `src-tauri/src/commands/collections/cmds.rs` (added refresh hooks, updated signatures)
- `src-tauri/src/commands/app/hotkey_cmds.rs` (added refresh hooks)
- `src-tauri/src/commands/app/settings_cmds.rs` (added refresh hooks)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (added refresh hooks)

## Goal

A single, robust in-game overlay that immediately reflects UI changes (toggles, keys, presets) and character detections.

## Impact

- Immediate UI-to-Game synchronization for all overlay artifacts.
- Improved reliability of character-specific keybind detection.
- Premium, unified look and feel for in-game documentation.
