# P2 redundant I/O baselines

## Context

P2 candidates needed reproducible measurements before changing safety-sensitive storage paths.

## Changes

- Added ignored, temporary-fixture benchmarks for no-op runtime-projection refreshes, duplicate-scan read paths, and import same-name target comparisons.
- Measurements keep production behavior unchanged and use temporary databases/directories only.

## Impacted Files

- `src-tauri/src/modules/workspace/adapters/sqlite/runtime_projection/mod.rs` (modified)
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_scanner_tests.rs` (modified)
- `src-tauri/src/modules/ingestion/application/import_batch/target_manifest_index.rs` (modified)

## Goal

Make redundant DB, WAL, and filesystem-read costs observable before implementing P2 optimizations.

## Impact

No production logic, command, schema, dependency, mutation guard, backup, atomic write, or reconcile behavior changed.

## Notes

Baseline results: no-op refresh of 1,000 projections made 2,000 row changes and grew WAL by median 111,240 bytes; duplicate fixtures exposed 1,310,720 bytes of deferred-eligible INI reads and 8,388,912 bytes of re-hash reads per scan; a 500-target import collision repeated a metadata observation with median 96 ms cost.
