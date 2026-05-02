# Deep Match browser shell fallback and legacy purge

## Context

Deep Match Scanner still had one end-to-end gap in browser import: jobs with canonical suggestions but no selected physical object could still fall back to shared `Other`, which left canonical relation unbound at object level. The product surface also still carried retired generic scan/organize commands and review-table leftovers.

## Changes

- Browser import now resolves the target object in this order: selected physical object, existing object with the same `matched_entry_key`, then a new physical object shell using the imported folder's physical name.
- Canonical relation is now persisted onto the resolved object shell during browser import placement, then scoped Disk Reconcile runs immediately after placement.
- Deep Match commit temp-target resolution now uses the same object-target helper as browser import so both flows follow the same physical-object reuse/create rules.
- Retired generic scanner/organizer command modules were removed from the product command graph and replaced with a narrow `scan_control_cmds` module for cancel state only.
- Scanner review UI no longer keeps the dead `onAutoOrganize` prop, and scanner type comments now describe the active Deep Match payloads.
- Added backend coverage for create/reuse behavior of physical object shells keyed by canonical relation.

## Impacted Files

- `src-tauri/src/services/browser/import_service.rs` (modified)
- `src-tauri/src/services/scanner/sync/helpers.rs` (modified)
- `src-tauri/src/services/scanner/sync/commit.rs` (modified)
- `src-tauri/src/services/scanner/tests/sync_tests.rs` (modified)
- `src-tauri/src/commands/mods/mod_import_cmds.rs` (modified)
- `src-tauri/src/commands/scanner/mod.rs` (modified)
- `src-tauri/src/commands/scanner/scan_control_cmds.rs` (added)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/scanner/core/mod.rs` (modified)
- `src-tauri/src/services/mods/organizer_ext.rs` (modified)
- `src-tauri/src/commands/scanner/scan_cmds.rs` (removed)
- `src-tauri/src/commands/scanner/organize_cmds.rs` (removed)
- `src-tauri/src/commands/scanner/tests/scan_cmds_tests.rs` (removed)
- `src-tauri/src/commands/scanner/tests/organize_cmds_tests.rs` (removed)
- `src-tauri/src/services/scanner/core/organizer.rs` (removed)
- `src/features/scanner/ScannerFeature.tsx` (modified)
- `src/features/scanner/components/ReviewTable.tsx` (modified)
- `src/features/scanner/components/useReviewTable.tsx` (modified)
- `src/features/scanner/components/useReviewTable.test.tsx` (modified)
- `src/types/scanner.ts` (modified)
- `.docs/flow.md` (modified)
- `.docs/requirements/req-25-scan-engine.md` (modified)
- `.docs/requirements/req-38-auto-organizer.md` (modified)

## Goal

Deep Match Scanner now preserves physical identity while still persisting canonical relation end-to-end, including browser imports that previously could fall into shared `Other`.

## Impact

- Browser import, scanner review, and deep-match commit now share one object-target resolution rule.
- User-facing product flows no longer depend on retired generic scan/organize commands.
- Physical folder names and paths remain untouched by canonical matching.
- Breaking change: retired generic scanner/organizer commands are no longer part of the product invoke surface.

## Notes

- Shared `Other` is still a valid runtime bucket for unmatched disk truth, but not the preferred sink for browser jobs that already carry a canonical suggestion.
