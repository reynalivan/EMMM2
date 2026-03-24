# Fix ObjectList [object Object] Render Error

## Context

Following recent DB and `tauri-specta` refactors, the UI began rendering `[object Object]` in the `ObjectList` when `get_objects` queries failed. The root cause was twofold:

1. The backend SQLite SQL failed to deserialize nullable JSON/TEXT columns (e.g., `metadata`, `tags`, `is_auto_sync`) into non-nullable Rust types.
2. The frontend blindly coerced custom Tauri error payloads like `{ "Database": "Column decode error..." }` by calling `String(error)`, resulting in `[object Object]`.

## Changes

- **Frontend Error Coercion**: Updated `ObjectListStates` wiring in `ObjectList.tsx` to safely inspect and serialize `typeof error === 'object'` instead of forcefully casting.
- **Backend Defaults**: Updated `get_filtered_objects` in `src-tauri/src/database/object_repo.rs` to use `COALESCE` for nullable columns (`metadata`, `tags`, `is_pinned`, `is_auto_sync`), aligning them exactly with the non-null Rust struct definitions.

## Impacted Files

- `src/features/object-list/ObjectList.tsx` (modified)
- `src-tauri/src/database/object_repo.rs` (modified)

## Goal

The `ObjectList` renders correctly without crashing, and backend JSON deserialization is resilient against legacy rows with `NULL` columns.

## Impact

- Prevented unhandled serialization crashes.
- Improved UI error visibility for developers when specific Tauri backend failures occur.
