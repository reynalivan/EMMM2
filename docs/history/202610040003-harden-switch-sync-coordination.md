# Harden switch synchronization coordination

## Context

Deferred synchronization must preserve disk-first responsiveness without broad rescans, stale optimistic overrides, unrelated repair isolation or endless retries after physical ownership is lost.

## Changes

- Proven scoped reconcile failures no longer escalate to full discovery; unavailable sources retain retry semantics.
- Confirmation ambiguity isolates overlapping operations; existing replacement identities require explicit repair rather than infinite retry. Independent operations retain the repair-hole checkpoint clamp.
- Terminal repair warnings are coalesced and bounded per game/root; disk observations remain intact and switches stay usable. Same-epoch reactivation resumes acknowledged idle records through the existing scheduler without historical rewrite replay or clearing successor intents.
- Added actual disk/SQLite/journal consumer regressions and correlated projection/runtime diagnostics. Mixed leaf/subtree runtime requests still conservatively select full composition.

## Impacted Files

- Backend: `src-tauri/AGENT.md`; `src-tauri/src/modules/reconciliation/adapters/tauri/{toggle_projection,runtime_sync}.rs`; `src-tauri/src/modules/reconciliation/application/toggle_projection.rs`; `src-tauri/src/modules/reconciliation/application/disk_reconcile/reconcile_tests.rs`; `src-tauri/src/modules/system/application/app/post_apply.rs`.
- Frontend: `src/features/workspace-runtime/actions/{useWorkspaceSwitchActions.ts,useWorkspaceSwitchActions.test.tsx,workspaceSwitchOps.ts,workspaceSwitchOps.test.ts}`; `src/shared/i18n/locales/{en,id,zh}/common.json`.
- Documentation: `docs/plans/stable-mod-switching/implementation_plan.md`; this history file.

## Goal

Disk acknowledgement stays the foreground completion boundary; DB, collection and runtime consumers converge through existing identity/revision-fenced ownership.

## Impact

No dependency, DB migration, journal format, public error DTO, scheduler or mutation lock added. No disk rollback on synchronization failure. Diagnostics are debug-level; native end-to-end latency is not inferred from fixture timing.

## Validation

Rust: 1,399 passed / 14 ignored. Frontend: 1,132 passed / 1 skipped, 205 files. TypeScript, full ESLint, architecture lint, Vite build, rustfmt, Clippy all-targets `-D warnings` and focused re-review passed. Scoped DB failure, multi-root confirmation, replacement-proof and same-epoch revisit regressions were test-first; no warnings were suppressed.

## Limits

Native UI click-to-paint/rage-click, mixed 10k bulk fairness/peak memory and full cold/network/ACL/power-loss acceptance remain separate release gates. No commit, push or installer build in this session.
