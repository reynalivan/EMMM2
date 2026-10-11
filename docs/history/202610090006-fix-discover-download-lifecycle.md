# Discover and Downloads lifecycle fixes

## Context

The audit found filename-policy bypasses after sanitization, incorrect pause/resume state, native startup/status races, cross-game tab publication, missing controls in small download lists, and leaked/unbounded native reservations.

## Changes

- Validate and display sanitized storage names; reject executable/script names after normalization.
- Byte progress no longer changes status. Native resume publishes an explicit running state and clears previous failure metadata.
- Persist startup before releasing the native deferral. Acknowledged UI-thread startup/control and one FIFO status writer preserve transition ordering; completed/canceled/imported rows reject regressions.
- Guard native tab publication by game/session generation, close stale webviews, and protect stale session restoration.
- Share transfer actions between table and virtual lists, including Pause, Resume, and Cancel for resumable failures.
- Bound native pending confirmations and live destination reservations; expire pending requests and release terminal/closed-tab ownership.
- Protect live/resumable files and history from rename/delete/retention until cancellation settles.

## Impacted Files

- `src/entities/browser/model/useBrowserStore.ts`, `useBrowserStore.test.ts`
- `src/pages/browser/utils/openBrowserTab.ts`, `openBrowserTab.test.ts` (added)
- `src/pages/browser/components/BrowserPage.tsx`, `DownloadManagerPanel.tsx`, `DownloadsPage.tsx`, `DownloadsPage.test.tsx`, `DownloadItemActions.tsx`, `DownloadConfirmationHost.tsx`, `DownloadConfirmationHost.test.tsx`
- `src/pages/browser/components/DownloadTransferActions.tsx` (added)
- `src/pages/browser/hooks/useDownloads.ts`, `useDownloads.test.ts`
- `src-tauri/src/modules/browser/adapters/sqlite/browser/downloads.rs`
- `src-tauri/src/modules/browser/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/browser/application/browser/mod.rs`, `download_handler.rs`, `download_service.rs`, `tests/download_service_tests.rs`
- `src-tauri/src/modules/browser/application/browser/native_download_state.rs`, `tests/native_download_state_tests.rs` (added)
- `src-tauri/AGENT.md`; this history file (added).

## Impact

No new dependency, migration, or frontend IPC payload. Native admission is bounded to 20 pending confirmations and 25 reserved operations overall; interrupted resumable operations retain their reservation until settled. One cleanup worker expires confirmations and retires closed-tab operations.

## Validation

- Reproduced filename, progress, missing-control, stale-tab, and SQL transition failures before fixing them.
- Frontend browser suite: 64 passed; production build and affected-file ESLint passed.
- Backend browser suite: 46 passed using SQLx offline metadata; strict Clippy, affected-file formatting, and diff whitespace checks passed.
- A rerun was temporarily blocked by concurrent watcher test edits; the final backend rerun passed after those definitions became available.
- Live WebView2 downloads and native application E2E were not run in this session.
