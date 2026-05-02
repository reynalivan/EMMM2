# Deep Match Legacy Cleanup

## Summary

- Fixed ObjectList archive stop flow to use `abort_extraction_cmd` instead of `cancel_scan_cmd`.
- Moved ObjectList bulk auto-organize to Deep Match Scanner preview flow instead of direct physical organizer mutation.
- Removed FE legacy `autoOrganizeMods` binding/hook surface and dropped the unused ScannerFeature organize action.
- Replaced stale browser permission names with active browser import command names.
- Removed public invoke exposure for legacy `start_scan`, `get_scan_result`, and `auto_organize_mods` user-facing surface from `lib.rs`.
- Renamed scanner review UI field usage from legacy `matchedObject` to `matchedAliasName`.

## Validation

- `pnpm exec tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run`
- `pnpm exec vitest run src/features/object-list/useObjectListHandlers.test.ts src/features/scanner/ScannerFeature.test.tsx src/features/scanner/components/useReviewTable.test.tsx src/features/file-watcher/hooks.test.ts src/lib/services/scanService.test.ts`

## Remaining Notes

- Legacy Rust modules for `scan_cmds` and `organize_cmds` still exist for internal tests/service reuse, but they are no longer wired into the normal FE invoke surface for Deep Match Scanner.
- Browser import fallback to shared physical `Other` still cannot safely attach canonical relation to a dedicated object row without a separate physical target strategy.
