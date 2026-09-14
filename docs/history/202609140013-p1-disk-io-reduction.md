# P1 disk I/O reduction

## Context

No-op metadata and INI updates still created replacement files, while import analysis reread the same folder and INI signals.

## Changes

- Existing `info.json` now skips its atomic replacement when a metadata update produces no semantic change; missing metadata still receives one final write.
- INI saves compare encoded output with current bytes after stale-source validation and skip temp, backup, and replacement files for no-op edits.
- Capped matcher INI reads stop after the configured prefix instead of reading the entire file first.
- Import analysis reuses one inspection folder snapshot and one request-local signal cache across classification and canonical matching.

## Impacted Files

- `src-tauri/src/modules/library/application/mods/info_json.rs` (modified)
- `src-tauri/src/modules/library/application/mods/tests/info_json_tests.rs` (modified)
- `src-tauri/src/modules/library/application/ini/write.rs` (modified)
- `src-tauri/src/modules/library/application/ini/tests/write_tests.rs` (modified)
- `src-tauri/src/modules/matching/application/deep_matcher/analysis/content.rs` (modified)
- `src-tauri/src/modules/matching/application/deep_matcher/tests/analysis/content_tests.rs` (modified)
- `src-tauri/src/modules/catalog/application/match_engine/classification.rs` (modified)
- `src-tauri/src/modules/catalog/application/match_engine/canonical_match.rs` (modified)
- `src-tauri/src/modules/ingestion/application/import_batch/analyze.rs` (modified)
- `src-tauri/src/modules/ingestion/application/import_batch/coordinator.rs` (modified)

## Goal

Reduce avoidable disk activity while preserving the filesystem as the source of truth.

## Impact

No command, schema, or dependency changes. Mutation validation, atomic writes when bytes change, rollback, and reconcile behavior remain intact. The matcher cache exists only for one import analysis item.

## Notes

`SQLX_OFFLINE=true cargo test --manifest-path src-tauri/Cargo.toml --lib` passed: 1069 passed, 4 ignored.
