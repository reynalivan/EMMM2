# MasterDB IPC Performance & Legacy Cleanup

## Context
Phase 2 of the Legacy & Backward Compatibility Audit. The MasterDB data flow to the frontend (used in the Auto Setup wizard) suffered from serialization overhead and retained old legacy JSON parsing fallbacks from Phase 1.

## Changes
- **Rust MasterDB Loading**:
  - Removed dynamic serde_json::Value (DOM) parsing and .clone() array allocations in load_master_db_json.
  - Created strict MasterDbPayload struct.
  - Replaced manual JSON stringification with typed Vec<DbEntry> return via Tauri IPC.
  - Eliminated the Value::Array fallback branch.
- **Frontend AutoSetupModal**:
  - Removed manual JSON.parse and legacy Array.isArray fallback branch.
  - Fixed a search filtering bug from Phase 1 where the filter incorrectly queried entry.tags (now correctly entry.aliases).
  - Switched useMasterDb hook to consume the unwrapped IPC DbEntry[] directly.

## Impacted Files
- src-tauri/src/services/scanner/master_db/mod.rs (modified)
- src-tauri/src/commands/objects/master_db_cmds.rs (modified)
- src/core/tauri/bindings.gen.ts (modified)
- src/features/object-list/hooks/useObjectQueries.ts (modified)
- src/features/object-list/hooks/useMasterDbSync.ts (modified)
- src/features/object-list/modals/AutoSetupModal.tsx (modified)
- src/features/object-list/modals/EditObjectTabAuto.tsx (modified)

## Goal
A strictly-typed, high-performance data bridge between the Master DB files and the React frontend with zero legacy parsing overhead or data fallbacks.

## Impact
- Significantly reduced memory allocations during Master DB loading (no JSON DOM clones).
- Faster Tauri IPC serialization since data is transferred as raw structures instead of double-encoded strings.
- Auto Setup search filter now correctly respects .aliases.
