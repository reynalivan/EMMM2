# Geometry-first KeyViewer sentinels

## Context

Raw hash harvesting allowed Body, Head, Dress, Face, and index hashes from one
mod to become separate observers or panels.

## Changes

- Preserved resource kind, callback slot, draw context, and INI source through
  harvest, catalog matching, and generated observers.
- Replaced rarity-based sentinel selection with geometry-first tier selection;
  all targets in the selected tier set one panel flag as an OR group.
- Made catalog-less fallback use terminal folder names, combine shared geometry
  into one panel, and reject ambiguous non-geometry collisions.
- Added catalog target draw context plus creator extraction/tests for GIMI key
  schema and v2 candidate reports.

## Impacted Files

- `src-tauri/src/modules/automation/application/keyviewer/harvester.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/matcher.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/generator/ini.rs` (modified)
- `src-tauri/src/modules/system/application/app/post_apply.rs` (modified)
- `src-tauri/src/modules/matching/application/deep_matcher/models/types.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/tests/` (modified)
- `docs/requirements/req-43-dynamic-keyviewer.md` (modified)
- `E:/code/projects/3dm-catalog-asset/scripts/maintain-runtime-targets.mjs` (modified)
- `E:/code/projects/3dm-catalog-asset/scripts/tests/maintain-runtime-targets.test.mjs` (added)

## Goal

One active character or fallback mod produces one KeyViewer panel without
runtime file scanning, while status remains available when no safe sentinel
exists.

## Notes

Creator output remains a reviewed v2 candidate. Publishing and signing a pack
still require complete explicit mappings and distribution-rights approval.
