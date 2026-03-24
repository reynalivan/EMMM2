## 202603230006-fix-lint-and-ts-tests.md

### Title

Fix Rust EOF lints and TypeScript test imports

### Context

Trunk linter was flagging extra newlines at EOF in several Rust files. `objectService.db.test.ts` had a broken relative import to a hook that was also unused, along with other unused types.

### Changes

- Trimmed trailing newlines to exactly one in five Rust files.
- Removed unused imports and fixed module resolution in `objectService.db.test.ts`.

### Impacted Files

- `src-tauri/src/commands/folder_grid/tests/mod_tests.rs` (modified)
- `src-tauri/src/commands/mods/tests/mod_meta_cmds_tests.rs` (modified)
- `src-tauri/src/services/app/tests/app_service_tests.rs` (modified)
- `src-tauri/src/services/mods/bulk_ops.rs` (modified)
- `src-tauri/src/services/objects/tests/query_tests.rs` (modified)
- `src/lib/services/objectService.db.test.ts` (modified)

### Goal

Pass lint checks and fix TypeScript compilation in test files.

### Impact

Clean build and test suite. No logic changes.
