# Command Registration & V2 Promotion

## Context

Gap analysis revealed 8 commands in `cmds.rs` not registered in `lib.rs`, causing runtime "Command not found" errors. 3 commands still had `v2_` prefix.

## Changes

- `cmds.rs`: Renamed `v2_update_collection` → `update_collection`, `v2_delete_collection` → `delete_collection`, `v2_get_collection_preview` → `get_collection_preview`
- `lib.rs`: Registered 8 missing commands (3 collection + 5 PIN)
- `app-commands.toml`: Removed 24 stale V1/v2-prefixed entries, added promoted names
- `useCollections.ts`: Updated 3 invoke calls to match promoted names
- `useSettings.ts`: Migrated `set_safe_mode_pin` → `set_pin`, `set_safe_mode_pin_with_recovery` → `set_pin` with client-side recovery code generation

## Impacted Files

- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (rewritten)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/hooks/useSettings.ts` (modified)

## Goal

All V2 commands registered and wired end-to-end. Zero stale legacy entries in permissions.

## Impact

- Collection update/delete/preview now functional at runtime
- All PIN operations (has/set/verify/clear/status) now functional
- No breaking changes to existing working commands
