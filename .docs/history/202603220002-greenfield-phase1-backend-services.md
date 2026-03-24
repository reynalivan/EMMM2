# Greenfield Phase 1: Backend Services + Tauri Commands

## Context

Phase 1 of the Collections + Privacy Corridor + Safe Mode greenfield redesign. Builds service orchestration and Tauri command layers on top of Phase 0 infrastructure.

## Changes

- `collection_service.rs`: 7 functions (list, create, update, delete, preview, apply via pipeline, undo)
- `corridor_service.rs`: 3 functions (get_state with dirty-flag computation, switch via pipeline, recompute_signature)
- `pin_service.rs`: 6 functions (get_status, set_pin with Argon2, verify_pin with lockout, verify_recovery, clear_pin, has_pin)
- `v2_cmds.rs`: 14 thin Tauri commands with `v2_` prefix (2 corridor, 7 collection, 5 PIN)
- `WatcherState` → `Arc<AtomicBool>` suppressor in switch pipeline (WatcherState not Clone)
- `safe_mode.is_safe` → `safe_mode.enabled` (matching actual config model)

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (added)
- `src-tauri/src/services/corridor_service.rs` (added)
- `src-tauri/src/services/pin_service.rs` (added)
- `src-tauri/src/services/mod.rs` (modified — registered 3 services)
- `src-tauri/src/commands/collections/v2_cmds.rs` (added)
- `src-tauri/src/commands/collections/mod.rs` (modified — registered v2_cmds)
- `src-tauri/src/lib.rs` (modified — 14 commands in generate_handler)
- `src-tauri/permissions/app-commands.toml` (modified — 14 commands whitelisted)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified — WatcherState → Arc<AtomicBool>)

## Goal

Complete backend service layer enabling frontend to call v2 commands for collections, corridor switching, and PIN management.

## Impact

- No breaking changes — v2 commands coexist with legacy commands
- Zero compile errors and warnings
- Pipeline pattern replaces monolithic 1177-line apply function
