# Unify All Preview/Modal Logic to Disk Discovery

## Context

Three different preview endpoints (Collection Preview Panel, Apply Collection Modal, Safe Mode Switch Modal) used inconsistent data sources. The Collection Preview Panel correctly used disk-based sub-folder discovery, while the Apply and Switch endpoints returned raw DB entries via UNION ALL, causing Object folder names to appear as mod names (e.g., "Ainoz" listed as a mod under "Ainoz OBJECT").

## Changes

- **Shared Helper:** Extracted `enrich_objects_with_disk_mods()` in `collection_service.rs` as a public reusable function that converts Object-only DB entries into enriched members by scanning physical depth-1 sub-folders on disk.
- **preview_apply:** Replaced UNION ALL query with Objects-only query + disk enrichment for both current and target member lists. Updated function signature to accept `mods_path`.
- **preview_switch:** Same refactor — Objects-only query + disk enrichment for both leaving and target members.
- **Tauri Commands:** Updated `preview_apply_collection` and `preview_corridor_switch` to resolve `mods_path` from `ConfigService` and pass it to their respective service functions.
- **Frontend Counts:** Changed "33 members" in Collection Preview header to "X active mods". Changed mod count badges in Apply Modal to count only `MemberKind::Mod` entries.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)

## Goal

All preview/modal UIs now show the same disk-accurate sub-mod view grouped by Object name, eliminating the confusing duplication of Object names as mod entries.

## Impact

- Apply Collection Modal now shows actual physical sub-folder names under each Object
- Safe Mode Switch Modal shows the same disk-enriched view
- Collection Preview shows "X active mods" instead of confusing raw member count
