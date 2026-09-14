# Align active shader conflict detection with GIMI runtime

## Context

The conflict notice could include INI or ShaderFixes files under nested `DISABLED*` folders, miss deeply nested INIs, remain stale after runtime-file mutations, and keep a newly changed conflict batch dismissed.

## Changes

- Added one recursive runtime-aware traversal for conflict INIs and ShaderFixes that prunes `DISABLED*`, skips symlinks, and ignores `desktop.ini`.
- Removed fixed discovery depth from active conflict detection.
- Enforced TextureOverride 8-hex and ShaderOverride 16-hex hashes and parsed the official `match_priority` key.
- Kept resource-hash conflicts conservative (`potential`) when unparsed draw/resource constraints prevent a definite conclusion.
- Invalidated active conflicts after INI writes, imports, and watcher runtime-file changes.
- Scoped toast dismissal to a deterministic conflict signature so changed conflicts notify again.
- Updated the notice/modal to say enabled-mod conflicts and show kind, certainty, source, section, namespace, condition, priority, index, and shader stage evidence.
- Added EN, ID, and ZH copy plus backend/frontend regression coverage.

## Impacted Files

Backend:

- `src-tauri/src/services/scanner/conflict/hash_scan.rs`
- `src-tauri/src/services/scanner/conflict/detect.rs`
- `src-tauri/src/services/scanner/conflict/tests/conflict_tests.rs`
- `src-tauri/src/services/mods/metadata.rs`
- `src-tauri/src/services/mods/tests/metadata_conflict_tests.rs`
- `src-tauri/src/commands/scanner/tests/conflict_cmds_tests.rs`

Frontend:

- `src/features/launch-bar/LaunchBar.tsx`
- `src/features/conflict-report/ConflictModal.tsx`
- `src/features/preview/hooks/usePreviewData.ts`
- `src/features/file-watcher/reconcileRefresh.ts`
- `src/features/mod-runtime/operations/sharedOperations.ts`
- Related tests and EN/ID/ZH locale files.

Documentation:

- `implementation_plan.md`
- `tasks/shader-conflict-active-notice-plan.md`
- `tasks/shader-conflict-active-notice-todo.md`
- `docs/3dmigoto_gap_status.md`

## Goal

Only conflicts contributed by files that GIMI can load from enabled mods appear in the persistent notice, and the notice stays fresh and actionable.

## Verification

- Targeted Vitest: 5 files, 19 tests passed.
- Scoped ESLint: passed.
- i18n lint: passed.
- Backend metadata conflict suite passed 10 tests after the traversal fix. The final full conflict rerun is blocked by unrelated object/workspace contract compile errors.
- Production build is blocked by unrelated stale frontend contract tests (`hasPin`, `safeMode`, and an old function signature).
