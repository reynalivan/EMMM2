# Reconcile and Collection Architecture Hardening - Execution Checklist

## Phase 0 - Characterization

- [x] T0.1 Add deterministic lock inversion and post-commit failure tests
- [x] T0.2 Add parent-object apply, task-finalization, rollback, and replace atomicity tests
- [x] T0.3 Add cross-game placeholder, partial-success refresh, and terminal-progress tests
- [x] Checkpoint 0: reviewer confirms tests model real audited failures

## Phase 1 - Concurrency and commit boundary

- [x] T1.1 Enforce `game_lock -> operation_lock` and remove pipeline reacquisition
- [x] T1.2 Return Applied plus pending effects after post-commit side-effect failure
- [x] Checkpoint 1: repeated concurrency/source-recovery tests and read-only review

## Phase 2 - Collection correctness

- [x] T2.1 Apply parent object state after terminal mod operations
- [x] T2.2 Finalize active baseline and recovery task atomically
- [x] T2.3 Persist exact rollback snapshot and prior baseline intent
- [x] T2.4 Save/replace collection in one transaction
- [x] Checkpoint 2: full Clean/Modified/Unsaved/retry/rollback matrix

## Phase 3 - Conflict, watcher, and source correctness

- [x] T3.1 Preserve existing DB state for unresolved conflict identities
- [x] T3.2 Scope rename confirmation and project unrelated roots
- [x] T3.3 Remove scoped suppression blind tail and bind repair generation to watcher session/root
- [x] T3.4 Harden source matching, fingerprint, and strict reconcile filesystem errors
- [x] Checkpoint 3: conflict/source/watcher external-change matrix

## Phase 4 - Single writer and performance

- [x] T4.1 Remove organizer direct projection/reference writes
- [x] T4.2 Consolidate grouped conflict resolution and delete legacy path
- [x] T4.2 Apply journal decision: remove JSON journal and reject unsupported cycles unless exception approved
- [x] T4.3 Add cheap global identity census plus true changed-root classification
- [x] T4.4 Reduce app mutations to targeted preflight plus one terminal reconcile
- [x] T4.4 Add thumbnail-only and root-event fast/escalation paths
- [x] Checkpoint 4: benchmark and reviewer confirm no new cache/drift source
  - Evidence: single-writer architecture test passed; 10k-folder snapshot benchmark passed in 6.26s.

## Phase 5 - Frontend and read model

- [x] T5.1 Remove actionable cross-game placeholder data
- [x] T5.2 Refresh successful partial mutations before surfacing errors
- [x] T5.2 Clear progress on every terminal result
- [x] T5.3A Add lightweight transactional collection runtime descriptor
- [x] T5.3B Update bindings/topbar/Collections page to use lightweight/lazy queries
- [x] T5.4 Collapse refresh mapping and deduplicate bulk invalidations
- [x] Checkpoint 5: frontend query/action consistency review
  - Evidence: 7 targeted suites / 64 tests passed together with `tsc --noEmit`.

## Phase 6 - Cleanup

- [x] T6.1 Remove dead runtime calculations and unused DTO fields
- [x] T6.2 Canonicalize fresh collection schema without corridor compatibility
- [x] T6.2 Remove stored-unsaved frontend branches after schema/runtime parity tests
- [x] T6.3 Remove inert reconcile queue and remaining legacy query/conflict code with usage proof
- [x] Checkpoint 6: fresh DB lifecycle, bindings, command registration, and read-only review
  - Evidence: canonical fresh-DB test and permission contract test passed; generated bindings are idempotent.

## Phase 7 - Verification

- [ ] T7.1 Disk/DB/projection/UI convergence matrix: in-app and external mutations
- [ ] T7.1 Collection Clean/Modified/Unsaved/Last Changes/recovery matrix
- [ ] T7.1 Conflict, source recovery, watcher overflow, and offline startup E2E
- [ ] T7.1 Performance fixture and scan-count assertions
- [ ] `rtk cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- [ ] `rtk cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
- [ ] `rtk cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `rtk pnpm test -- --run`
- [ ] `rtk pnpm lint`
- [ ] `rtk pnpm i18n:lint`
- [ ] `rtk pnpm build`
- [ ] Targeted `rtk pnpm test:e2e`
- [ ] `rtk git diff --check`
- [ ] Final command registration/permission/bindings/docs audit
- [ ] Final read-only correctness, concurrency, regression, and simplification review

## Subtask dispatch guard

- [x] Every task has one explicit owner and file boundary
- [x] No two production agents edit reconcile/apply pipeline files concurrently
- [x] Every worker receives the shared-worktree/non-revert handoff preamble
- [x] Every worker reports exact tests and residual risks
- [x] Reviewer gate is complete before advancing past each checkpoint
