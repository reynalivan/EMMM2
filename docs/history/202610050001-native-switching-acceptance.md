# Native switching acceptance (paused)

## Context

Execute approved Section 9 against actual Windows disk/WebView2, preserving disk-first acknowledgement and minimal architecture. User requested pause before completion.

## Changes

- Isolate non-production AI credential services; keep the production legacy service. Earlier stale E2E reset may have removed an existing AI key; user informed.
- Prove fresh E2E binary/identifier; restrict cleanup to owned PIDs and registered fixtures. Use checkpoint waits and JSON primitive IPC transport.
- Correct canonical-versus-ordinary Windows paths in projection, single/bulk trust, watcher classification and authority dirty scopes. Keep physical names/identity/no-overwrite proof.
- Extend bounded local timing evidence and native regression/acceptance fixtures; no new writer, schema, dependency or public command.

## Impacted files

- Backend: `src-tauri/AGENT.md`, `src-tauri/src/lib.rs`, `src-tauri/src/platform/security/credential_store.rs`, `src-tauri/src/shared/path_key.rs`.
- Commands: `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`, `src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs`.
- Native benchmark: `src-tauri/src/modules/mutation/native_tests.rs`.
- Projection: `src-tauri/src/modules/reconciliation/application/toggle_projection.rs`, `src-tauri/src/modules/reconciliation/adapters/tauri/toggle_projection.rs`.
- Reconcile: `src-tauri/src/modules/reconciliation/application/disk_reconcile/emit.rs`, `reconcile.rs`, `reconcile_tests.rs`, `path_classifier.rs`, `watcher_batch.rs`; same directory's `orchestrator/state.rs`, `orchestrator/tests.rs`.
- Harness: `wdio.conf.ts`; `tests/e2e/support/driverLifecycle.ts`, `fixtures.ts`, `app.ts`, `ipc.ts`, `data.ts`.
- Tests: `src/tests/driverLifecycle.test.ts`, `src/tests/nativeFixtures.test.ts`; `tests/e2e/specs/switching-stability.e2e.ts`, `switching-bulk.e2e.ts`, `phase3-mod-ops.e2e.ts`.
- Evidence: `docs/plans/stable-mod-switching/implementation_plan.md`, `native-acceptance-report.md`, `native-storage-ack-raw.log`, `native-webview-results.json`; this history file.

## Goal and impact

Native stability spec passed, including 1,000 last-wins intents and disabled-ancestor current-state exclusion without changing saved membership. Windows namespace regressions were RED then GREEN. Storage-only benchmark passed with outliers retained; native bulk responsiveness remains unproven after the latest fix. Interrupted owned fixtures remain for recovery/inspection.

## Validation and resume

Vitest: 1,141 passed/1 skipped. Frontend/E2E TypeScript, ESLint/architecture lint, Vite/native debug build and diff-check passed. Latest focused Rust watcher-authority regression, seven rename-hint tests and serialized-request regression passed. Last full Rust run: 1,404 passed/4 failed/14 ignored; separator failures subsequently fixed, final full rerun pending. Focused review found no confirmed issue. Resume native bulk/timing, then full Rust, Clippy and final evidence; manual overlay/Explorer/cold/network gates remain explicit. No commit/push/installer.
