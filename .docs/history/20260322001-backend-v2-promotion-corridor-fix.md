# Backend V2 Promotion & Corridor Service Fix

## Context

After Greenfield System Redesign, `corridor_service.rs` referenced a non-existent `mod_repo` module in `repo/` and used an incorrect `CorridorError::Database` variant. The `useSafeModeToggle.test.tsx` tested a removed `setSafeModeWithToast` API.

## Changes

- `corridor_service.rs`: Removed `mod_repo` import → inlined SQL query for enabled mods. Changed `CorridorError::Database` → `CorridorError::Db`. Fixed `CollectionError` → `CorridorError` mapping in `preview_switch`.
- `collection_repo.rs`: Cleaned legacy "collection_repo_v2" comment header.
- `useSafeModeToggle.test.tsx`: Complete rewrite to match V2 hook state-machine API (no more `setSafeModeWithToast`).
- `useCollections.test.ts`: Aligned test data to V2 `CorridorSnapshot` (was using `CorridorRuntimeSnapshot`).

## Impacted Files

- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src/hooks/useSafeModeToggle.test.tsx` (rewritten)
- `src/features/collections/hooks/useCollections.test.ts` (modified)
- `src/hooks/useFolders.ts` (modified)

## Goal

Backend compiles and tests pass with zero errors.

## Impact

- No breaking changes to runtime behavior
- Test coverage now matches V2 API surface
