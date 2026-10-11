# Fix clarification and destination mapping

## Context

The audit found ambiguous catalog-key/object selection, destructive existing-object classification, divergent preview/commit paths, invalid high-confidence matches, cross-game alias leakage, inconsistent destination priority, and destination-parent races.

## Changes

- Resolve existing targets by exact game-owned object ID; reject conflicting keys, invalid create combinations, and malformed batch target modes.
- Separate display folder names from complete paths; share object-directory and subpath resolution between review and commit.
- Preserve existing object classification and child mod categories; initialize classification only for canonical creation, including recovery retries.
- Reject empty normalized name scoring, retain explicit zero-score targets, and preserve backend priority/persisted choices in the wizard.
- Scope learned aliases by game type without replacing the cache-generation guard.
- Retain destination ancestry and pinned directories through publication. Create new components atomically; use handle-relative no-overwrite publication and preserve ownership during failed compensation.
- Leave interrupted targets without durable ownership proof untouched and mark repair-required. Restore the catalog discard command's explicit permission entry.

## Impacted Files

- Catalog: `src-tauri/src/modules/catalog/adapters/sqlite/object/matching.rs`; `src-tauri/src/modules/catalog/application/match_engine/{destination.rs,tests.rs}`.
- Ingestion: `src-tauri/src/modules/ingestion/adapters/sqlite/import_batch/{mod.rs,tests.rs}`; `src-tauri/src/modules/ingestion/application/import_batch/{coordinator.rs,mod.rs,relocation.rs,placement.rs,placement_tests.rs}`.
- Mutation: `src-tauri/src/modules/mutation/application/workspace_mutation/{import_commit.rs,import_commit_tests.rs}`.
- Platform: `src-tauri/src/platform/fs/{file_utils.rs,rename.rs}`.
- Cache: `src-tauri/src/modules/workspace/application/scanner/master_db/cache.rs`; `src-tauri/src/modules/workspace/application/scanner/tests/master_db_cache_tests.rs`.
- Frontend: `src/features/match-wizard/components/{ImportBatchWizardDestinationPanel.tsx,ImportBatchWizardDestinationPanel.test.tsx}`.
- Permission/documentation: `src-tauri/permissions/app-commands.toml`; `src-tauri/AGENT.md`; this history file.

## Goal and Impact

The reviewed destination stays authoritative through placement without reclassifying unrelated existing mods. Recovery fails closed instead of moving an unproven payload. No dependency, migration, public IPC payload, commit, or worktree added; unrelated edits preserved. Placement into existing objects no longer implicitly rewrites classification or learns aliases through that writer; explicit classification remains available.

## Validation

- Regression-first checks demonstrated wrong-object selection, subpath/path drift, invalid input, empty-name scoring, zero-score target loss, and UI ordering failures before fixes.
- Frontend: 25 focused tests passed; production build and targeted ESLint/Prettier passed.
- Backend: import batch 56 passed (1 manual benchmark ignored), matching 12, alias cache 6, native import commit 11, filesystem helpers 9, and command permission registry 1 passed in targeted runs.
- Final strict Clippy (`--lib --locked --offline -- -D warnings`), formatting, and whitespace checks passed.
- Independent review challenged the publication and compensation error paths; final correction retains owned-target evidence and requires repair rather than claiming unchanged state.
- Windows junction/replacement, actual pinned publication, and one-character component tests ran natively. Linux descriptor/compensation paths were statically reviewed but not executed on this Windows-only host; no live application E2E performed.

## API Reference

Native relative rename semantics checked against Microsoft's [NtSetInformationFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile) and [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information) contracts.
