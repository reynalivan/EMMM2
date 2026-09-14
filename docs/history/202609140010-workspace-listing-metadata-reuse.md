# Workspace listing metadata reuse

## Context

Opening a nested Mods Grid listing built both the root and active-folder views, and each view separately loaded all owner descriptors and safety classifications.

## Changes

- Load owner and safety metadata once per workspace read.
- Reuse that immutable metadata for the root Object List mapping and active Mods Grid listing, including shallow recovery mode.

## Impacted Files

- `src-tauri/src/modules/workspace/application/explorer/listing/owners.rs` (modified)
- `src-tauri/src/modules/workspace/application/explorer/listing/mod.rs` (modified)
- `src-tauri/src/modules/workspace/application/workspace/mod.rs` (modified)

## Goal

Avoid duplicate SQLite reads while preserving the current filesystem listing and enrichment results.

## Impact

Nested folder navigation now performs one owner read and one safety read instead of repeating both for root and active listings. No cache lifetime, payload, sorting, or freshness behavior changed.

## Notes

`cargo check` and 16 workspace view-model tests passed.
