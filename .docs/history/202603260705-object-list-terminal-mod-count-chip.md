# Object list terminal mod count chip

## Context

Object list row counts were still using raw mod rows, so the sidebar chip did not match collection tree semantics and could not show `active/total` terminal mods correctly.

## Changes

- Reworked object list count loading in `object_repo` from raw SQL `COUNT(*)` to terminal-root aggregation.
- Collapsed `FlatModRoot`, `ModPackRoot`, and `VariantContainer` into one count each per terminal root.
- Ignored pure `ContainerFolder` rows from total counts.
- Excluded descendants under disabled container folders from active counts while still keeping them in total potential counts.
- Changed the object row badge from separate numbers to one muted chip in `(active/total)` format.
- Added focused tests for row rendering and terminal-count aggregation behavior.

## Impacted Files

- `src-tauri/src/repo/object_repo.rs` (modified)
- `src-tauri/src/commands/objects/tests/object_cmds_tests.rs` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/object-list/ObjectRowItem.test.tsx` (added)

## Goal

Object list rows now show terminal mod counts that align with preview tree semantics, using a single chip like `(2/10)`.

## Impact

- Object sidebar counts are now more accurate for variant containers, mod packs, flat mods, and disabled-container branches.
- No API shape change was needed; existing consumers still read `mod_count` and `enabled_count`.
- Rust runtime tests remain blocked in this environment by `STATUS_ENTRYPOINT_NOT_FOUND`, so backend verification reached compile-only plus frontend runtime tests.
