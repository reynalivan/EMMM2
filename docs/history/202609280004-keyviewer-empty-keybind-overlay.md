# KeyViewer empty state for mods without switcher keys

## Context

KeyViewer generated a blank panel for catalog matches without keybinds and excluded keyless fallback mods even when they had a detectable runtime resource.

## Changes

- Generated panels now show “No switcher key on this mod” when no usable key/back binding exists.
- Fallback grouping keeps mods with a detectable observer even if their keybind list is empty; mods without an observer remain excluded.
- Added regression coverage for empty, blank, mixed-source, and fallback cases.

## Impacted Files

- `src-tauri/src/modules/automation/application/keyviewer/generator/keybind_text.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/tests/generator/keybind_text_tests.rs` (modified)
- `src-tauri/src/modules/system/application/app/post_apply.rs` (modified)
- `src-tauri/AGENT.md` (modified)
- `docs/history/202609280004-keyviewer-empty-keybind-overlay.md` (added)

## Goal

Keep the KeyViewer overlay visible with a clear empty state when the active mod has no switcher key.

## Impact

Keyless fallback mods with runtime detection now produce panels. No filesystem mutation or database schema change.
