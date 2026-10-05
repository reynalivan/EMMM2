# Close switch preparation and echo-accounting gaps

## Context

Resumed approved native acceptance. Rapid/bulk tests exposed false watcher dirt, an unapplied preparation failure, and expensive rename-hint filtering.

## Changes

- Align scoped storage/echo admission and Windows physical namespaces; avoid double consumption and false duplicate dirt.
- Repair preparation-time uncertainty once under the original guards, then validate the unchanged plan. Failed proof aborts the untouched journal.
- Finish split-then-stitched echo evidence after native proof; index rename-hint coverage without changing filtering semantics.
- Wait for visible native dropdown options; retain bounded path-free diagnostics and honest failed-run evidence.

## Impacted Files

- Backend: `src-tauri/AGENT.md`; library `adapters/tauri/mod_bulk_cmds.rs`; workspace `adapters/tauri/workspace_cmds.rs`.
- Reconciliation: `application/disk_reconcile/{rename_confirmation.rs,orchestrator/state.rs,orchestrator/tests.rs}`.
- Watcher: `workspace/application/scanner/{watcher/event_filter.rs,watcher/mod.rs,watcher/suppressor.rs,watcher/lifecycle.rs,tests/watcher_tests.rs}`.
- Native harness: `tests/e2e/specs/switching-stability.e2e.ts`.
- Evidence: `docs/plans/stable-mod-switching/{implementation_plan.md,native-acceptance-report.md,native-webview-results.json,native-webview-results-20261006.json}`; this history file.

Backend groups above are relative to `src-tauri/src/modules/`.

## Goal and Impact

Preserve disk-first responsiveness and ownership without new queues, dependencies or schema. Rust 1,415 passed/14 ignored; frontend 1,141 passed/1 skipped; types, lint/architecture, rustfmt, Clippy and diff checks passed. Native stability passed 7/7; fresh debug Tauri/Vite builds passed. Targeted regressions were RED then GREEN; focused review found no outstanding correctness issue.

Bulk 100/1,000 improved in one run, but 10k preparation and later repeated conservative scans prevented acceptance closure. Windows `Modify(Any)` cannot safely be assumed harmless. Narrow namespace/ancestor proof needs separately reviewed scope approval; no broad suppression added. Interrupted owned fixtures retained; completed stability roots cleaned by the harness. Manual OS/runtime/storage gates remain open. No commit, push or installer.
