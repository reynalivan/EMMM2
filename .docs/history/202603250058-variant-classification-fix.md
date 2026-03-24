# Variant Container Classification Fix & Preview Sync

## Context
1. **Classifier Bug**: Mod folders containing structurally valid variant subfolders (e.g. `1.school uniform`, `2.no skirt`) but possessing a ZERO-byte inner root INI (like an empty `merged.ini` or none at all) were failing the VariantContainer check. The legacy classification logic enforced an early short-circuit `if !has_mod_ini` right near the top of the function, which incorrectly caused folders lacking a root mod INI but containing valid subfolders to fall back down to `ContainerFolder`.
2. **Active Preview Sync**: Clicking an actively selected Unsaved Collection showed the raw database ID (`unsaved_YYYYMMDDXXX`) in the Collection Preview Panel header because it lacked the same localization mapping applied earlier to the list items.

## Changes
- Re-architected `classifier.rs`:
  - Moved the `has_mod_ini` short-circuit *below* the `VariantContainer` explicit rule check.
  - The classifier now correctly evaluates a `VariantContainer` purely off the count and nature of its subfolders independently of whether the root folder possesses its own valid Mod INI, perfectly aligning with AC-11.3.5.
- Updated `listing.rs`:
  - Added `VariantContainer` to the `self_is_mod` boolean expression so the FolderGrid API payload formally recognizes it as a terminal mod when directly queried.
- Fixed `CollectionPreviewPanel.tsx`:
  - Implemented the `t('context.unsaved')` translation map on the preview header so the synced Active Preset matches the "Unsaved Preset" text displayed in the `CollectionList`.

## Impacted Files
- `src-tauri/src/services/explorer/classifier.rs` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)

## Goal
Variant Container folders (like `AmberCN_Vest_school uniform_Togle_v2` with empty `merged.ini`) will no longer incorrectly render as navigable subfolders in the FolderGrid, instead acting as solid mod items in the frontend. The preview panel will also stay visually synchronized when an Unsaved Preset is currently Active.

## Impact
- Subfolders of Variant Containers mapped this way are now properly shielded from navigation.
- The UI mapping for Unsaved Presets is ubiquitous across all collection panels.
