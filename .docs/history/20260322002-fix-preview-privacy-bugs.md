# Fix Preview & Privacy Mode Logic Bugs

## Context

Collection Preview and Corridor Switch modals were skipping objects with zero active mods, first-time Safe/Unsafe switch was disabling all ObjectList items, and unsaved presets had a static generic label.

## Changes

- **Object visibility decoupled from mods**: UNION ALL queries in `collection_service.rs`, `corridor_service.rs`, `switch_pipeline.rs` changed from `JOIN mods m ON m.object_id = o.id WHERE ... m.status = 'ENABLED'` to `WHERE o.folder_path NOT LIKE 'DISABLED %'`. Objects are now visible even with zero active mods.
- **First-time corridor baseline**: `corridor_service.rs` → `preview_switch` now returns all physically enabled Objects as `is_enabled: true` baseline when no saved target state exists (first switch).
- **Depth-1 filter removed**: `switch_pipeline.rs` `disable_leaving` no longer filters out depth-1 folders. All enabled mods are properly disabled during corridor switch.
- **Preview panels show empty objects**: `CollectionPreviewPanel.tsx` and `ModeSwitchConfirmModal.tsx` now pass all object members (regardless of `is_enabled`) to grouping, while only filtering mod members to enabled.
- **Unsaved preset label**: `corridorLabels.ts` changed static "Unsaved Preset" to dynamic `Unsaved YYYYMMDDHHMM` using `buildUnsavedPresetLabel()`.
- **SQL bind order fix**: `switch_pipeline.rs` had 4 binds for 3 placeholders; corrected to 3 binds.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/lib/corridorLabels.ts` (modified)

## Goal

Accurate preview for all privacy modes. Objects always visible. First switch defaults all objects ON. Unsaved presets show last snapshot timestamp.

## Impact

- Preview panels now show ALL objects, including those with no active mods
- First Safe→Unsafe switch correctly enables all ObjectList items
- Corridor switch properly disables all mods (no depth-1 leak)
- Collection naming more informative
