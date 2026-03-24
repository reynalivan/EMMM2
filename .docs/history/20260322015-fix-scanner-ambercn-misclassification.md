# Title

Fix Scanner AmberCN Misclassification

## Context

The scanner (`walker.rs`) previously only scanned depth 1 of the `Mods` directory. This incorrectly treated parent organization folders like `ambercn` as mods themselves. The Deep Matcher then incorrectly tied `ambercn` to the `amber` Object. `AmberCaballeraExploradora` inside `ambercn` was ignored entirely, blending concepts between Object and Mod.

## Changes

- Rewrote `walker::scan_mod_folders` and `walker::scan_specific_folders` to use `walkdir` for deep recursive scanning (max depth 6).
- Integrated `classifier::classify_folder` into the walker. The scanner now only yields specific `NodeType` folders (`ModPackRoot`, `FlatModRoot`, `VariantContainer`) as valid Mod candidates, stopping recursion upon finding an actual mod.
- Adjusted `sync/commit.rs` `commit_scan_results` logic. When assigning an object to a deep mod (e.g. depth > 1), the first part of the relative path (i.e. Depth-1 chunk) is dynamically extracted to serve as the logical `object_folder_path` and initial fallback Object Name.

## Impacted Files

- `src-tauri/src/services/scanner/core/walker.rs` (modified)
- `src-tauri/src/services/scanner/sync/commit.rs` (modified)

## Goal

The scanner accurately drills down to find real Mod folders while still preserving the depth-1 parent directory as its logical `Object` container grouping in the database.

## Impact

- `ambercn` will now properly be registered as an `Object`.
- Actual mods like `AmberCaballeraExploradora` will populate as the distinct variants under that object.
- Initial filesystem scans may take slightly longer due to recursive walking, but it correctly mirrors the filesystem index constraints.
