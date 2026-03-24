# Fix ObjectList Deserialization Error

## Context
The user encountered an error loading the Object List: `objectlist on load error occurred while decoding column "custom_skins": invalid type: sequence, expected a map at line 1 column 0`.
This occurs because `custom_skins` (and sometimes `hash_db`) in the SQLite database may contain an empty array (`[]`) from old migrations or invalid inserts, but the Rust structs `CustomSkinsPayload` and `HashDbPayload` are wrappers around `HashMap`. `serde_json` fails to parse `[]` into a `HashMap`.

## Changes
- Updated the `sqlx::Decode` implementations for `HashDbPayload` and `CustomSkinsPayload` in `src-tauri/src/database/models.rs`.
- Added a check: if the database text is `"[]"` or empty, the decoder now gracefully returns `Self::default()` (an empty `HashMap`) instead of failing deserialization.

## Impacted Files
- `src-tauri/src/database/models.rs` (modified)

## Goal
Make the SQLite deserialization resilient against legacy or malformed JSON array strings for map-based payloads, preventing the entire ObjectList from crashing.

## Impact
- `listObjects` queries will now succeed even if corrupted/legacy records exist in the database.
- Completely backwards-compatible with standard JSON object `"{}"` formats.
