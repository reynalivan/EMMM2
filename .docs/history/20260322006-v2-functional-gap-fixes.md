# V2 Functional Gap Fixes

## Context

Post-audit fixes to align preview panel and collection detail display with V2 architecture.

## Changes

- **`corridor_service.rs::preview_switch`** — Removed stale SYSTEM-disabled mod query fallback. Now uses collection-based resolution (active_collection_id → undo_collection_id → empty) matching `restore_target`.
- **`CollectionPreviewPanel.tsx`** — Replaced inline member rendering with shared `ModGroupList` component. Gains: consistent object grouping, enabled/disabled badges, thumbnails, expand/collapse, unsafe count indicators.
- **Gaps 7.3-7.4** — Verified corridor query invalidation already present: 7 occurrences in `useFolders.ts`, 6 in `useFolderMutations.ts`. No changes needed.

## Impacted Files

- `src-tauri/src/services/corridor_service.rs` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)

## Goal

All V2 functional features match or exceed V1 capabilities. Preview panels, collection details, and mod toggle flows are fully operational.

## Impact

- Corridor switch preview no longer queries raw SYSTEM-disabled mods
- Collection detail panel now matches corridor switch preview in display richness
