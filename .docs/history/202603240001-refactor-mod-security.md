## Refactor Mod Management Security & Type Safety

### Context

The mod management system required a unified security model and consistent error handling to align with the Tauri-Specta migration and prevent potential path traversal vulnerabilities.

### Changes

- Integrated `PathGuard` into all core mod services (`core_ops`, `bulk`, `trash`, `metadata`, `info_json`) for strict path validation.
- Transitioned backend error handling from `String` to structured `AppError` enum for JSON-compatible frontend reporting.
- Injected `ConfigService` into command layers and services to provide game configuration context.
- Restored truncated `PostApplyService` pipeline logic for KeyViewer synchronization.
- Standardized bulk operation results to return nested `AppError` details for failure reporting.

### Impacted Files

- `src-tauri/src/domain/errors.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/services/mods/bulk.rs` (modified)
- `src-tauri/src/services/mods/trash.rs` (modified)
- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)
- `src-tauri/src/services/app/post_apply.rs` (modified)
- `src-tauri/src/commands/mods/*_cmds.rs` (modified)
- `src-tauri/src/services/hotkeys/actions.rs` (modified)

### Goal

Enforce end-to-end security and type-safe error handling across the mod management lifecycle.

### Impact

- Standardized error responses across all mod-related UI components.
- Hardened file system operations against directory traversal.
- Fully synchronized DB/Disk/UI state transitions.
