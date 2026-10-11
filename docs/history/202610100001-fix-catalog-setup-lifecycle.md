# Fix catalog setup lifecycle

## Context

Six catalog audit findings affected GIMI queries, stale caches, refresh safety, import rollback, and abandoned preview storage.

## Changes

- Accept game type `0`; cancel old catalog reads and refresh catalog, search, suggestion, thumbnail, and object consumers.
- Guard MasterDB publication with an invalidation generation.
- Validate refresh before clearing catalog-owned thumbnail references; serialize catalog operations.
- Retain backup until provenance commits; restore old pack on failure and keep tokens retryable. Share swap logic with signed updates.
- Add idempotent discard IPC, three-review admission limit, ten-minute expiry, and startup/periodic orphan cleanup restricted to non-linked UUID staging directories.
- Block cancel/backdrop/Escape during install; release previews on settings unmount and show actionable failures inside review.

## Impacted Files

- Frontend: `src/widgets/object-sidebar/hooks/useObjectQueries.ts`, `useObjectQueries.test.tsx`; `src/pages/settings/components/tabs/CatalogTab.tsx`, `CatalogTab.test.tsx`; `src/pages/settings/lib/catalogQueries.ts`, `catalogQueries.test.ts`; `src/shared/api/tauri/bindings.gen.ts`.
- Backend: `src-tauri/src/lib.rs`; `src-tauri/src/modules/catalog/adapters/tauri/master_db_cmds.rs`; `src-tauri/src/modules/workspace/application/scanner/master_db/{mod.rs,cache.rs,catalog_import.rs,catalog_import_cleanup.rs,catalog_update.rs,pack_swap.rs,test_support.rs}`; `src-tauri/src/modules/workspace/application/scanner/tests/master_db_cache_tests.rs`.
- Documentation: `src-tauri/AGENT.md`; this history file.

## Goal and Impact

Catalog installation and retry remain consistent without discarding user thumbnail metadata or leaving unbounded staging. No migration or dependency added. Unrelated workspace edits preserved.

## Validation

- Frontend: 17 targeted tests passed; production build, ESLint, Prettier, and diff whitespace checks passed.
- Backend: final 33 master DB and 2 catalog-refresh tests passed; SQLite deferred-constraint failure verifies rollback and retry. Clippy (`--lib --locked --offline -- -D warnings`) and targeted Rustfmt checks passed. A transient unrelated filesystem test import error resolved before the successful rerun.
- No live GitHub download/native picker E2E or abrupt-process-crash recovery test performed. Filesystem/SQLite operations are rollback-safe for reported runtime failures, not a distributed crash transaction.
