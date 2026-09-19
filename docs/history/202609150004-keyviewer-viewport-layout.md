# KeyViewer viewport layout

## Context

The character panel position changed with match order and match count instead of staying tied to the game viewport.

## Changes

- Replaced per-match grid geometry with one normalized viewport box shared by every character panel.
- Changed text alignment to left, moved the shortcut status banner upward, and increased both overlay scales to 1.35.
- Bumped the KeyViewer layout revision so existing runtime artifacts are republished.

## Impacted Files

- `src-tauri/src/modules/automation/application/keyviewer/generator/ini.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/tests/generator/ini_tests.rs` (modified)

## Goal

Keep KeyViewer placement stable across character changes and match-set changes while improving readability.

## Impact

Character overlays now share one viewport anchor. If multiple character detections render in the same frame, they use the same box by design.

## Notes

The layout follows 3DMigoto text renderer normalized coordinates and `TextParams` left alignment semantics.
