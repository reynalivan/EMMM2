# Deep Match Scanner Alignment

Date: 2026-03-25 21:30 WIB

## Summary

Deep Match Scanner now preserves physical object identity and stores canonical relation data separately.

Key changes:
- added canonical relation fields for Deep Match output and object summaries
- kept physical `objects.name` and `objects.folder_path` unchanged during commit
- updated commit/preview tests to assert canonical relation storage instead of physical rename
- removed physical rename behavior from manual `Sync with DB` quick actions in ObjectList and FolderGrid
- moved object-list archive/drop directory creation off direct `@tauri-apps/plugin-fs` imports by using app commands
- added a minimal command test for `score_candidates_batch_cmd`

## Verification

Passed:
- `pnpm exec tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run`
- `pnpm exec vitest run src/lib/services/scanService.test.ts src/features/settings/tabs/GamesTab.test.tsx src/features/object-list/useObjectListHandlers.test.ts src/features/scanner/ScannerFeature.test.tsx`

## Residual Gaps

- legacy generic scan commands (`start_scan`, `get_scan_result`) still exist for older/internal flows; they are no longer the main Deep Match Scanner UI path
- `match_object_with_db` quick action is now enrichment-only for manual single-item sync and no longer renames physical folders, but it is still separate from the full preview/commit Deep Match Scanner flow
- base migration `20260323000000_init.sql` still contains the legacy `match_object_id` column on `import_jobs`; fresh installs are corrected by follow-up migration `20260325220000_deepmatch_canonical_relation.sql`
