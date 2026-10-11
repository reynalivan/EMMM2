# Storage Optimizer lifecycle and cancellation

## Context

Start failures after page unmount left scan state active. Late cancellation could still persist a report, root comparison folded DISABLED aliases, and resolve selections survived game/report changes.

## Changes

- The scan hook owns startup, session events, and failure cleanup independently of page subscriptions.
- Cancel notifications come from accepted terminal events, not cancellation IPC acknowledgement.
- A single mutex orders backend cancellation against report commit. Each run owns a fresh token; cancellation rejects after commit begins, and the session is released before terminal events.
- Check cancellation after final hashing and grouping; an accepted cancellation also blocks report persistence.
- Reuse exact location keys for report roots, preserving DISABLED directory names.
- Reset selections and confirmation on game/scan changes; reject empty or pending confirmation requests.

## Impacted Files

- `src/features/scanner/StorageOptimizerPage.tsx`, `StorageOptimizerPage.test.tsx`
- `src/features/scanner/hooks/useDedup.ts`, `useDedup.test.ts`
- `src/features/scanner/stores/useDedupScanStore.ts`
- `src/features/scanner/components/DuplicateReport.tsx`, `DuplicateReport.test.tsx`
- `src-tauri/src/modules/duplicates/adapters/sqlite/dedup/mod.rs`, `tests.rs`
- `src-tauri/src/modules/duplicates/adapters/tauri/tauri.rs`
- `src-tauri/src/modules/duplicates/application/dedup/scanner.rs`
- `src-tauri/AGENT.md`
- This history file (added).

## Goal and Impact

Lifecycle state stays recoverable across navigation. Cancellation has a defined persistence boundary, roots retain their physical spelling, and resolve decisions belong to the displayed report. No new dependency, migration, or IPC payload. Disk reads still stop cooperatively at scan checkpoints.

## Validation

- Frontend scanner suite: 91 tests passed; production build and affected-file ESLint passed.
- Backend duplicates suite: 52 passed, 1 manual benchmark ignored using SQLx offline metadata.
- Strict Clippy, Rust formatting, and diff whitespace checks passed.
- Regression tests demonstrated the frontend and root-comparison failures before fixes; session tests verify both sides of the cancellation/commit boundary.
- Native application E2E was not run in this session.
