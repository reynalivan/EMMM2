# Stabilize Mod Inbox import flow

## Context

Mod Inbox did not refresh reservations after opening a batch, and the demo could not complete the review-to-move path.

## Changes

- Refresh Mod Inbox after batch reservation so pending sources cannot be selected again.
- Enable the import wizard in demo mode with in-memory Mod Inbox batch commands.
- Expose mutation readiness from the workspace and reject new Mod Inbox batches until disk-watcher authority is current.
- Update import E2E coverage for automatic categorization, explicit collision clarification, review acknowledgement, and watcher settlement.

## Impacted Files

- `src/pages/mod-inbox/ModInboxPage.tsx` (modified)
- `src/pages/mod-inbox/ModInboxPage.test.tsx` (modified)
- `src/demo/commands.ts` (modified)
- `src/demo/commands.test.ts` (added)
- `src/demo/workspace.ts` (modified)
- `src/app/entrypoint/App.tsx` (modified)
- `src/features/import-batches/ImportBatchWizardHost.tsx` (modified)
- `src/features/workspace-runtime/hooks/useWorkspaceReadModels.test.tsx` (modified)
- `src/shared/api/tauri/bindings.gen.ts` (modified)
- `src-tauri/src/modules/workspace/domain/workspace/view.rs` (modified)
- `src-tauri/src/modules/workspace/application/workspace/mod.rs` (modified)
- `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs` (modified)
- `src-tauri/src/modules/ingestion/adapters/tauri/tauri.rs` (modified)
- `tests/e2e/support/data.ts` (modified)
- `tests/e2e/specs/phase5-import.e2e.ts` (modified)

## Goal

The Inbox → review/clarification → move flow now keeps selection state accurate and waits for safe native mutation readiness.

## Impact

Batch creation can briefly ask the user to retry while the disk watcher settles; it no longer reserves sources for a move the backend would reject.
