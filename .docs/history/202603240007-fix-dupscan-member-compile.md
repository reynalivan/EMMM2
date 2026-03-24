# Fix DupScanMember Compilation Error

## Context

A recent refactor caused a compilation error `missing field version in initializer of DupScanMember` in `src-tauri/src/services/scanner/dedup/scanner.rs:363`. In addition, `mod_id` was left unpopulated (hardcoded to `None`) despite the availability of `path_to_mod_id` mapping from the scanner context.

## Changes

- **Updated `scanner.rs:build_groups()` signature**: Passed `path_to_mod_id: &HashMap<String, String>` mapping to allow ID resolution.
- **Fixed `DupScanMember` Instantiation**:
  - Provided `version: None` explicitly (no upstream version tracking is available at this stage).
  - Populated `mod_id` dynamically via a lookup: `path_to_mod_id.get(&folder_path).cloned()`.

## Impacted Files

- `src-tauri/src/services/scanner/dedup/scanner.rs` (modified)

## Goal

Restore application build integrity while properly populating the DB keys (`mod_id`) onto the dedup scan reports so the frontend can properly manage these entities.

## Impact

- Resolved one compiler error (`E0063`).
- Re-enabled proper DB id mapping on `DupScanMember` entities out of the scanner.
