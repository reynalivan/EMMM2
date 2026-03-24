## Title

Unified In-Game Overlay Synchronization & Zero-Leak Policy

## Context

The previous overlay system used an obsolete rendering method and lacked a reliable synchronization mechanism, leading to stale keybind displays and "phantom" entries when mods were disabled or characters removed.

## Changes

- **Unified Pipeline**: Migrated all overlay rendering to the 3DMigoto `help.ini` (GIMI PrintText) standard.
- **Zero-Leak Cleanup**: Implemented mandatory `remove_dir_all` on active keybind artifacts before regeneration to prevent stale data leaks.
- **100% Sync Coverage**: Integrated `trigger_overlay_refresh` into all state-altering commands, including bulk operations, metadata updates, safety toggles, and conflict resolution.
- **Mod-Grouped Labeling**: Updated the generator to group keybinds by their source mod folder name, providing clear `[Mod: Name]` headers in the overlay.
- **FS Watcher Hook**: Linked the filesystem watcher to the overlay pipeline to ensure manual Explorer changes (rename/delete) reflect instantly in-game.

## Impacted Files

- `src-tauri/src/services/app/post_apply.rs` (modified: unified pipeline & cleanup)
- `src-tauri/src/services/keyviewer/generator.rs` (modified: grouped labeling & help.ini format)
- `src-tauri/src/commands/mods/mod_bulk_cmds.rs` (modified: added refresh triggers)
- `src-tauri/src/commands/mods/mod_meta_cmds.rs` (modified: added refresh triggers)
- `src-tauri/src/commands/mods/conflict_cmds.rs` (modified: added refresh triggers)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified: added refresh triggers)
- `.docs/requirements/req-42-ingame-hotkeys.md` (updated)
- `.docs/requirements/req-43-dynamic-keyviewer.md` (updated)
- `.docs/flow.md` (updated with Section 10: Overlay Sync Flow)

## Goal

Achieve a seamless, real-time synchronization between the EMM2 application state and the in-game overlay, ensuring that what the user sees in the app is exactly what appears in the game, with zero stale data.

## Impact

- **Performance**: High-efficiency hash harvesting and atomic file writes ensure minimal overhead during refreshes.
- **UX**: Eliminates "phantom" keybinds and provides clear mod context via grouping.
- **Reliability**: Robust against manual filesystem changes and complex bulk operations.
