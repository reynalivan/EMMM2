# Refactor: Mod Management Security & Architecture

## Context

A comprehensive security and architectural audit was required to enforce path security, standardize error handling, and ensure atomic database-filesystem synchronization across the mod management system.

## Changes

- **Security**: Migrated all filesystem-touching commands (Preview, Metadata, Thumbnails) to `PathGuard` and `AppError`.
- **Protocol**: Implemented `emmm://` URI scheme in `lib.rs` and `ThumbnailCache` for secure, validated asset serving to the frontend.
- **Atomicity**: Integrated `SuppressionGuard`, `OperationLock`, and transaction-based updates (SQLx) in all mod toggle and pinning operations.
- **Pinning**: Implemented end-to-end mod and object level pinning with persistent `info.json` synchronization.
- **Stabilization**: Resolved ~20 compilation errors in the command, service, and pipeline layers to achieve a clean build.

## Impacted Files

### Commands

- `src-tauri/src/commands/mods/mod_meta_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src-tauri/src/commands/mods/preview_cmds.rs` (modified)
- `src-tauri/src/commands/objects/object_cmds.rs` (modified)
- `src-tauri/src/commands/scanner/conflict_cmds.rs` (modified)
- `src-tauri/src/commands/folder_grid/mod.rs` (modified)

### Services

- `src-tauri/src/services/mods/metadata.rs` (modified)
- `src-tauri/src/services/mods/info_json.rs` (modified)
- `src-tauri/src/services/mods/trash.rs` (modified)
- `src-tauri/src/services/mods/variant_service.rs` (modified)
- `src-tauri/src/services/images/thumbnail_cache.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)

### Infrastructure & Core

- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/repo/mod_repo.rs` (modified)
- `src-tauri/src/repo/object_repo.rs` (modified)
- `src-tauri/Cargo.toml` (modified)

## Goal

Achieve a secure, atomic, and type-safe architecture that prevents race conditions and data corruption while strictly enforcing path access boundaries.

## Impact

- **Security**: Significantly enhanced by the `emmm://` protocol and `PathGuard`.
- **Stability**: Atomic operations eliminate state discrepancies between DB and disk.
- **Maintainability**: Unified error handling (`AppError`) and clean `cargo check`.
