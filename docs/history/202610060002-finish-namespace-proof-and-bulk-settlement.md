# Finish namespace proof and bulk settlement

## Context

Ordinary watcher dirt delayed physical prefix switches. Folded renames could create false expiry gaps, and 10k projection settlement blocked subsequent switches.

## Changes

- Capture request-local complete native/canonical parent proof before waits; validate under the durable lease and every rename attempt. Preserve original source ownership, no-overwrite and strict snapshot barriers.
- Yield ordinary watcher batches through existing nonblocking catch-up/retry; keep dirty evidence for projection.
- Use the original validated session/proof for echo registration. Account for maximal folded rename chains; pending callbacks settle only after commit proof or release on abort/expiry.
- Register bulk foreground priority before identity resolution. Finalize exact projection batches in one atomic durable journal snapshot before sidecar cleanup; invalid/repair IDs cannot partially complete.

## Impacted Files

- `src-tauri/AGENT.md`, `src-tauri/src/platform/fs/file_utils.rs`.
- Library: `adapters/tauri/mod_bulk_cmds.rs`, `application/mods/{bulk/toggle.rs,core_ops/toggle.rs}`.
- Workspace: `adapters/tauri/workspace_cmds.rs`, `application/workspace/switch.rs`, `application/scanner/watcher/{lifecycle.rs,suppressor.rs}`.
- Reconciliation: `application/disk_reconcile/orchestrator/{entry.rs,state.rs,tests.rs}`, `adapters/tauri/toggle_projection.rs`.
- Mutation: `{coordinator.rs,journal.rs,tests.rs}`.
- Evidence: `docs/plans/stable-mod-switching/{implementation_plan.md,native-acceptance-report.md,native-webview-results-namespace-20261006.json}`; this history file.

Backend module groups are relative to `src-tauri/src/modules/`.

## Goal and Impact

Disk-first responsiveness without new queues, caches, dependencies, schema or weaker durability. Native **9/9** passed, including 10k bulk/checkpoint and post-bulk last-wins. The unrelated 10k leaf improved **2,665.1 -> 297.2 ms**; 314-operation journal settlement measured **336 ms**. Bulk 10k itself took **165.55 s**. Trusted first-toggle disk observation **215.52 ms**, next-frame proxy **10.8 ms**.

Regressions were RED then GREEN; full Rust **1,425 passed/14 ignored**, frontend **1,141 passed/1 skipped**. Types/build, lint/architecture, rustfmt, Clippy, formatting, diff-check and focused review passed. Repeated latency SLA, actual paint, real-game overlay/Explorer and special storage gates remain unverified. Failed attempts retained in evidence; owned completed fixtures cleaned. No commit, push or installer.
