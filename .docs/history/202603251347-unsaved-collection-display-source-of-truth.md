# Unsaved collection display source of truth

## Context

Unsaved collection names were rendered from different sources. The collections list and preview used a generic label, the topbar dropdown used the raw DB name, and the topbar trigger could generate a fresh timestamp label at render time.

## Changes

- Added a shared `getCollectionDisplayName` helper so user-facing collection labels resolve from one rule set.
- Removed timestamp label generation from corridor label fallback.
- Updated topbar trigger and dropdown to use the shared helper and corridor unsaved metadata instead of raw names.
- Updated collection list and preview header to use the same shared helper.
- Extended `CorridorSnapshot` with `active_collection_is_unsaved` so the topbar can resolve unsaved display names without guessing from other queries.
- Added focused frontend tests for the helper and topbar label alignment.
- Added a backend test covering unsaved active collection metadata in corridor state.

## Impacted Files

- `src/lib/corridorLabels.ts` (modified)
- `src/lib/corridorLabels.test.ts` (added)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.test.tsx` (added)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/types/collection.ts` (modified)
- `src/features/collections/hooks/useCollections.test.ts` (modified)
- `src-tauri/src/domain/corridor.rs` (modified)
- `src-tauri/src/repo/corridor_repo.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)

## Goal

Unsaved collections now display as `Unsaved Preset` consistently in the collection list, topbar trigger, and topbar dropdown, while named collections keep their real names.

## Impact

- Topbar no longer invents timestamp-based unsaved labels.
- Corridor state now exposes explicit unsaved metadata for the active collection.
- No DB migration or storage rename was added.

## Notes

- One existing Vitest assertion in `src/features/collections/hooks/useCollections.test.ts` still fails because it expects query-cache mutation behavior unrelated to this naming fix.
