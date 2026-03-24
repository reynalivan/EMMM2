# System Migration Gap Fixes

Addressing architectural gaps in KeyViewer generation, hotkey variant cycling, and unified post-apply triggers.

## Context

After the database refactor and Tauri-Specta migration, several critical pipelines (KeyViewer harvester/matcher) were unwired, and hotkey variant cycling was a stub. This change unifies these triggers into a single service.

## Changes

- **Post-Apply Service**: Created `run_post_apply_tasks` to unify signature recompute, KeyViewer generation, conflict caching, and status banner updates.
- **Variant Cycling**: Implemented characters-scoped variant discovery and cycling in `HotkeyManager`.
- **Pipeline Integration**: Wired post-apply tasks into `ApplyPipeline` and `SwitchPipeline`.
- **Frontend Polish**: Improved cache invalidation in `useDedup` to ensure UI-DB sync after resolution.

## Impacted Files

- `src-tauri/src/services/app/post_apply.rs` (added)
- `src-tauri/src/services/mods/variant_service.rs` (added)
- `src-tauri/src/services/app/mod.rs` (modified)
- `src-tauri/src/services/mods/mod.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)
- `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src/features/scanner/hooks/useDedup.ts` (modified)

## Goal

Establish a robust, automated pipeline for in-game overlay generation and variant management.

## Impact

- **Automation**: KeyViewer.ini is now automatically updated on any mod change.
- **Performance**: Unified triggers reduce redundant DB writes and FS scans.
- **UX**: Hotkey variant cycling is now fully functional.
