# Import payload manifest single walk

## Context

Import analysis validated a staged source tree and then immediately walked it again to build its full-content manifest.

## Changes

- Added a validated source-manifest builder that performs payload security checks before metadata reads and hashing in the manifest traversal.
- Import analysis now uses that builder instead of invoking validation and manifest construction separately.
- Commit revalidates the source with the same builder before moving files; generic target manifests remain unchanged.

## Impacted Files

- `src-tauri/src/modules/ingestion/application/import_batch/payload_manifest.rs` (modified)
- `src-tauri/src/modules/ingestion/application/import_batch/analyze.rs` (modified)
- `src-tauri/src/modules/mutation/application/workspace_mutation/import_commit.rs` (modified)

## Goal

Each analyzed staged item now performs one full metadata traversal for source validation and manifest construction while preserving fresh disk validation at commit.

## Impact

- Removes one complete `WalkDir` metadata pass per analyzed item before its unavoidable full-content hashing.
- No command, schema, dependency, mutation guard, atomic move, rollback, or reconcile behavior changed.

## Notes

`rustfmt` passed for the new manifest module. Focused Rust tests are currently blocked by an unrelated duplicate `mod tests` declaration in `workspace/application/workspace/switch.rs`.
