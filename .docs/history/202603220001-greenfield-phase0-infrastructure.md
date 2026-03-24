# Greenfield Architecture Phase 0: Schema + Domain + Pipeline Infrastructure

## Context

Phase 0 of the Collections + Privacy Corridor + Safe Mode greenfield redesign. This establishes the new DB schema, domain types, repository layer, and pipeline infrastructure that will coexist with the legacy system during migration.

## Changes

- New 4-table normalized DB schema (`corridor`, `collection_v2`, `collection_member`, `pin_config`) replacing 7 fragmented legacy tables
- Inline data migration from old tables during schema creation
- `domain/` module with pure types: `CorridorId`, `CorridorState`, `CorridorSnapshot`, `SwitchResult`, `Collection`, `CollectionKind`, `MemberKind`, `CollectionMember`, `ApplyResult`, `PinConfig`, `PinStatus`
- Typed error hierarchy: `CorridorError`, `CollectionError`, `PinError`, `AppError` using `thiserror`
- `repo/` module with clean CRUD: `corridor_repo` (6 ops), `collection_repo_v2` (13 ops with batch `replace_members`), `pin_repo` (5 ops)
- `pipeline/` module: `apply_pipeline` (8-step orchestrator replacing monolithic 1177-line `apply.rs`) and `switch_pipeline` (4-step corridor switch)
- 8 individual pipeline steps: validate, resolve_target, resolve_current_state, compute_diff, snapshot_state, batch_rename, batch_db_update, update_corridor
- All new code uses runtime `sqlx::query` (not compile-time macros) for coexistence with legacy DB
- `blake3` used for signature hashing (existing dependency, replaces planned `md5`)

## Impacted Files

- `src-tauri/migrations/011_greenfield_schema.sql` (added)
- `src-tauri/src/lib.rs` (modified — registered 3 new modules)
- `src-tauri/src/domain/mod.rs` (added)
- `src-tauri/src/domain/corridor.rs` (added)
- `src-tauri/src/domain/collection.rs` (added)
- `src-tauri/src/domain/errors.rs` (added)
- `src-tauri/src/domain/pin.rs` (added)
- `src-tauri/src/repo/mod.rs` (added)
- `src-tauri/src/repo/corridor_repo.rs` (added)
- `src-tauri/src/repo/collection_repo_v2.rs` (added)
- `src-tauri/src/repo/pin_repo.rs` (added)
- `src-tauri/src/pipeline/mod.rs` (added)
- `src-tauri/src/pipeline/apply_pipeline.rs` (added)
- `src-tauri/src/pipeline/switch_pipeline.rs` (added)
- `src-tauri/src/pipeline/steps/mod.rs` (added)
- `src-tauri/src/pipeline/steps/validate_corridor.rs` (added)
- `src-tauri/src/pipeline/steps/resolve_target.rs` (added)
- `src-tauri/src/pipeline/steps/resolve_current_state.rs` (added)
- `src-tauri/src/pipeline/steps/compute_diff.rs` (added)
- `src-tauri/src/pipeline/steps/snapshot_state.rs` (added)
- `src-tauri/src/pipeline/steps/batch_rename.rs` (added)
- `src-tauri/src/pipeline/steps/batch_db_update.rs` (added)
- `src-tauri/src/pipeline/steps/update_corridor.rs` (added)

## Goal

Establish the complete greenfield infrastructure that runs in parallel with legacy code, enabling Phase 1 (service layer) and Phase 2 (frontend) to build on top.

## Impact

- No breaking changes — all new code coexists with existing system
- Zero compile warnings after cleanup
- Legacy commands continue to work unmodified
