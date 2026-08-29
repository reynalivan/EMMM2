# Remove Backward-Compatibility Legacy Code

## Context
Audit of backward-compatibility code to reduce runtime overhead and dead branches.

## Changes

### 1. Object & Metadata Parsing
- parse_custom_skins: stripped Value::Object branch (legacy map coercion); direct serde_json::from_str
- HashDbPayload::decode / CustomSkinsPayload::decode: removed array guard fallback
- MasterDb::from_json: removed legacy array arm; only accepts {entries:[...]} format
- All test JSON fixtures updated to new object format

### 2. Database Collections & Jobs
- update_collection (crud.rs): removed active cache-only load_projected_collection_state repair call
- settings/mod.rs: removed has_legacy_scan_results sqlite_master query + conditional DELETE
- Added migration 20260829000000_cleanup_legacy.sql: DROP TABLE IF EXISTS scan_results

### 3. Keyviewer Paths
- resolve_d3dx_ini_path: removed legacy_candidate game-exe parent lookup + 2 blocking is_file() I/O calls

### 4. Frontend / API Naming
- DbEntry.tags renamed to DbEntry.aliases in Rust struct + bindings.gen.ts
- useMasterDbSync.ts: removed tags->aliases mapping shim; DbEntryFull drops tags field

### 5. Not Changed (Intentional)
- normalizer.rs regex kept - 3DMigoto creates DISABLED-/DISABLED_/DISABLEDFoo folders actively

## Impacted Files
- src-tauri/src/services/objects/classification.rs (modified)
- src-tauri/src/services/objects/tests/classification_tests.rs (modified)
- src-tauri/src/domain/models.rs (modified)
- src-tauri/src/services/scanner/deep_matcher/state/master_db.rs (modified)
- src-tauri/src/services/scanner/master_db/tests.rs (modified)
- src-tauri/src/services/scanner/core/tests/organizer_tests.rs (modified)
- src-tauri/src/services/scanner/deep_matcher/tests/acceptance_result_tests.rs (modified)
- src-tauri/src/services/scanner/deep_matcher/tests/models/acceptance_tests.rs (modified)
- src-tauri/src/services/objects/classification_batch.rs (modified)
- src-tauri/src/services/collection/crud.rs (modified)
- src-tauri/src/repo/settings/mod.rs (modified)
- src-tauri/src/services/keyviewer/generator/reload_key.rs (modified)
- src-tauri/migrations/20260829000000_cleanup_legacy.sql (added)
- src/core/tauri/bindings.gen.ts (modified)
- src/features/object-list/hooks/useMasterDbSync.ts (modified)
- src/features/collections/components/CollectionList.test.tsx (modified)
- src/features/collections/components/CollectionPreviewPanel.test.tsx (modified)
- src/shared/components/layout/top-bar/ContextControls.test.tsx (modified)

## Goal
All backward-compat runtime branches removed. Data flows through typed structs with no dynamic JSON coercion or speculative disk probes.

## Impact
- Faster JSON parsing in classification, master DB loading, deep matcher pipeline
- Saves 2 blocking is_file() I/O calls per Keyviewer init
- Removes 1 sqlite_master existence query + conditional DELETE per full reset
- Removes 1 extra collection state load round-trip per collection rename
- Breaking: DB rows with custom_skins as old object map format will fail to parse

## Notes
- All 863 backend unit tests pass after changes
