# Phase 16: Fix Dirty State and Preview Panel Issues

## Context

Two critical UX bugs were reported in the V2 Collection System:

1. **Dirty State Unreliable:** After saving a collection, toggling a mod did not immediately trigger the "Unsaved Preset" badge in the Topbar. Sometimes saving a new collection showed it as "Unsaved Preset" instantly.
2. **Preview Panel Grouping Broken:** Mismatched names and missing folder grouping (e.g., Albedo showing as the raw child mod name instead of the parent folder) when clicking Apply Collection.

## Changes

- **Fix 1:** Modified `collection_service::create_collection`, `collection_service::preview_apply`, and `snapshot_state::snapshot` to query BOTH `objects` and `mods` using a `UNION ALL`. This guarantees that `MemberKind::Object` rows actually exist in the DB for named collections and undo snapshots, fixing the frontend grouping logic (`groupMods.ts`) where fallback logic was overwriting object names.
- **Fix 2:** Modified `corridor_service::recompute_signature` to calculate the Dirty State using the exact same `UNION ALL` of `objects` and `mods`.
- **Fix 3:** Replaced SQLite `ORDER BY` sorting with deterministic Rust byte-sorting (`enabled_keys.sort()`) across both `corridor_service` and `collection_service` to ensure exact hash matching and fix the false positive Dirty States.

## Impacted Files

- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/pipeline/steps/snapshot_state.rs` (modified)

## Goal

Ensure exact 1:1 parity between how the Topbar calculates the current system footprint vs how Collections save their footprint, while ensuring Collections capture both Mod and Object levels for UI presentation.

## Impact

- The Topbar Dirty State badge should now be highly accurate — detecting exactly when the physical enabled mods/objects drift from the active collection cache.
- The Apply Collection Preview dialog will natively group nested mods under their parent Object names (e.g., Albedo -> variant 1, variant 2) instead of discarding the Object context.
