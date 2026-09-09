# Ignore Disabled Conflict Files

## Context

Enabled Mod Conflicts still reported INI files explicitly disabled through a `DISABLED` filename prefix.

## Changes

- Conflict traversal now excludes disabled file entries as well as disabled directories.
- Added coverage for `DISABLEDStelle.ini` so it cannot create an enabled-mod conflict.

## Impacted Files

- `src-tauri/src/modules/workspace/application/scanner/conflict/hash_scan.rs` (modified)
- `src-tauri/src/modules/library/application/mods/tests/metadata_conflict_tests.rs` (modified)

## Goal

Only runtime-loaded files contribute to Enabled Mod Conflicts.

## Impact

Disabled INI files and disabled ShaderFix replacement files are ignored during conflict scanning. No data or folder state is changed.
