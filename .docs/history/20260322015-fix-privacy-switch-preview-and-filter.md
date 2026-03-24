# Fix Privacy Switch Preview & Collection Filter

## Context

The UI was presenting data incorrectly in two specific areas:

1. The **Collection Details** preview panel was showing all physical disk sub-mods (both disabled and enabled) instead of just the active snapshot.
2. The **Privacy/Safe Mode Transition** preview modal was dumping all active mods into a single "Uncategorized" group instead of grouping them by their parent `Object` name, due to a missing mapping layer and an inherited `o.folder_path_key` SQL syntax error inside the transition backend.

## Changes

- **Collection UI Filter:** Implemented a direct `.filter(m => m.is_enabled)` block inside the frontend `useMemo` computation within `CollectionPreviewPanel.tsx` prior to passing the payload into the mod grouping hook.
- **Privacy SQL Overhaul:** Reworked `preview_switch` in `corridor_service.rs` to replace the simple `mods` extraction with a comprehensive `UNION ALL` query. This identical architecture (pulled from Collection Saves) joins both `objects` and `mods` arrays by `object_id`, preserving parent object context.
- **MemberKind Translation:** Updated `corridor_service.rs` struct map to correctly ingest `kind_str` variables from the SQL query and parse them accurately into `MemberKind::Mod` and `MemberKind::Object` enums.

## Impacted Files

- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)

## Goal

To guarantee that saved Collection snapshots strictly render enabled state subsets to the user exactly as requested, while completely repairing the Safe Mode Transition modal to visually bucket mods identically to the primary Collections workflow.

## Impact

- Collection Preview panel exclusively renders sub-mods that are toggled on.
- The Privacy Mode transition modal securely routes preview metadata, properly presenting distinct Object group headers encapsulating respective transitioning mods.
- The application bypasses the previous unmapped column SQL parsing errors entirely during transition previews.
