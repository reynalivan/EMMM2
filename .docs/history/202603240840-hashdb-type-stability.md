### Title

Refactor hash_db to Strong Typing and Defensive SQL Validation

### Context

Legacy empty string values (`""` or `" "`) in SQLite's `hash_db` and `custom_skins` JSON columns caused fatal `serde_json` decoding `EOF` panics when `sqlx` attempted to parse them during Auto-GC (`get_filtered_objects`). The frontend TypeScript definitions were also loosely typed (`string | null`), leading to inconsistencies.

### Changes

- **Backend Strong Typing**: Introduced explicit `HashDbPayload` and `CustomSkinsPayload` structs in `models.rs` using `std::collections::HashMap` instead of dynamic `serde_json::Value`.
- **Defensive SQL**: Wrapped all JSON column selections in `object_repo.rs` with `CASE WHEN json_valid(o.hash_db) = 1 THEN o.hash_db ELSE NULL END` to definitively shield Rust from corrupted raw payloads.
- **Frontend Sync**: Hand-updated `src/types/object.ts` to `Record<string, string[]> | null` and added robust JSON stringify/parse serialization logic to the UI form bindings in `useEditObjectForm.ts`.
- **DB Migration**: Created `20260324000000_scrub_json_columns.sql` to actively delete invalid string remnants currently stored in users' local DBs.

### Impacted Files

- `src-tauri/src/database/models.rs` (modified)
- `src-tauri/src/database/object_repo.rs` (modified)
- `src-tauri/migrations/20260324000000_scrub_json_columns.sql` (added)
- `src/types/object.ts` (modified)
- `src/features/object-list/hooks/useEditObjectForm.ts` (modified)

### Goal

The backend is completely immune to `hash_db` decoding panics regardless of what legacy non-JSON text the column holds, and the TS frontend has strict, accurate typings for metadata payloads.

### Impact

- Resolves all Auto-GC crashes related to `.json` EOF parsing.
- Introduces stricter parsing handling on UI forms.
- Re-executes a new SQL migration natively inside `pnpm tauri dev`.
