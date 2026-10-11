# Fix browser and duplicates regressions

## Context

Ten confirmed findings affected download ownership, terminal state and duplicate identity/reporting.

## Changes

- Native completion uses owned staging, handle-bound no-overwrite publication/compensation and ordered closed-tab settlement.
- Fallback ownership protects live history; final byte counters persist without throttling.
- Download rename/rollback cannot overwrite raced files; uncommitted native paths fail closed.
- Extended filenames decode correctly; title callbacks preserve history across visits.
- Exact duplicates compare complete manifests; whitelist reports every allowed pair; physical locations keep distinct IDs/safety.

## Impacted Files

Browser (modified unless marked added):

- `src-tauri/src/modules/browser/application/browser/download_handler.rs`
- `src-tauri/src/modules/browser/application/browser/download_service.rs`
- `src-tauri/src/modules/browser/application/browser/native_download_state.rs` (added)
- `src-tauri/src/modules/browser/application/browser/browser_service/metadata.rs`
- `src-tauri/src/modules/browser/adapters/sqlite/browser/downloads.rs`
- `src-tauri/src/modules/browser/application/browser/tests/download_service_tests.rs`
- `src-tauri/src/modules/browser/application/browser/tests/native_download_state_tests.rs` (added)

Duplicates (modified unless marked added):

- `src-tauri/src/modules/duplicates/application/dedup/signals.rs`
- `src-tauri/src/modules/duplicates/application/dedup/scanner.rs`
- `src-tauri/src/modules/duplicates/application/dedup/grouping.rs`
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_scanner_tests.rs`
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_grouping_tests.rs`
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_signals_tests.rs` (added)

Verification/documentation:

- `tests/e2e/specs/browser-duplicates-regressions.e2e.ts` (added)
- `src-tauri/AGENT.md` (modified)
- This history file and `.bug-hunter/browser-duplicates-fix/` scope, checkpoint, helper, review and verification artifacts (added).

## Goal and Impact

Preserve intervening files and truthful terminal/report state without new dependencies, migrations or public IPC changes. Persistent DB failure retains payload for repair instead of risking unrelated files.

## Verification

1,785 Rust tests passed; 32 ignored. Targeted browser: 57 passed. Strict Clippy, app/E2E typecheck, scoped lint/format and fresh native build passed. Six private-profile native cases passed; owned fixtures/processes cleaned. Enabled/DISABLED two-row ID/safety preconditions are verified in Rust integration, not fresh native onboarding, which intentionally excludes conflicted aliases. Two earlier native fixture failures are retained in the execution report. No commit was created.
