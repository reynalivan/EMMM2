# Implementation Roadmap: Reconcile and Collection Architecture Hardening

## 1. Objective

Menutup gap correctness, concurrency, recovery, performance, dan frontend consistency yang ditemukan pada audit reconcile/collection tanpa menambah polling daemon, filesystem cache kedua, generic saga, atau event bus baru.

Target akhirnya adalah satu alur yang mudah dipahami:

```text
UI / Watcher / Startup
        |
        v
Mutation Coordinator
(game lock -> operation lock)
        |
        +--> targeted preflight
        +--> filesystem mutation, jika ada
        |
        v
Disk Discovery (blocking, scoped/full)
        |
        v
Projection Writer (satu SQLite transaction)
        |
        +--> path/reference healing
        +--> runtime projection
        |
        v
Apply Finalizer (active baseline + task status transaction)
        |
        v
Idempotent Runtime Effects
        |
        v
Satu terminal result -> React Query invalidation
```

## 2. Non-negotiable Invariants

1. Disk pada configured Mods directory adalah authority untuk existence, path, dan enabled/disabled.
2. Database adalah projection, metadata store, collection baseline, dan recovery metadata; database tidak boleh membuat ulang folder yang hilang kecuali ada explicit user operation.
3. Semua mutation memakai satu lock order: `game_lock -> operation_lock`.
4. Projection writer adalah satu-satunya writer untuk mod/object path, stable identity, enabled state, runtime projection, dan collection reference healing.
5. Error sebelum projection commit berarti tidak ada projection baru. Error sesudah commit tidak boleh dilaporkan seolah commit tidak terjadi.
6. Conflict hanya melindungi identity/scope yang ambigu. Folder lain tetap dapat converge.
7. Collection apply dinyatakan selesai hanya bila disk mutation, projection, active baseline, dan recovery task sudah final.
8. Scoped watcher suppression tidak boleh membuang perubahan eksternal asli.
9. SAFE/UNSAFE tetap classification/view filter dan tidak memengaruhi runtime signature atau filesystem mutation.
10. React Query tetap presentation cache; tidak ditambahkan persistent filesystem cache, polling, atau cache DB kedua.

## 3. Scope Decisions

### Included

- Lock ordering and deadlock prevention.
- Reconcile pre/post-commit contract.
- Parent object state during collection apply.
- Exact apply rollback metadata and atomic finalization.
- Atomic save/replace collection.
- Conflict projection, rename confirmation scope, watcher suppression, and source-directory classification.
- Single-writer migration for organizer and conflict resolution.
- True scoped discovery and removal of duplicate full scans.
- Cross-game frontend cache correctness, partial-success refresh, runtime DTO/read consistency.
- Legacy corridor/unsaved/dead-cache cleanup after behavior stabilizes.

### Explicitly excluded

- Generic saga engine.
- Persistent filesystem cache.
- Background polling daemon.
- New application event bus.
- Broad rewrite of scanner, collection, or React Query architecture.
- Repo-wide formatting or unrelated cleanup.

### Conflict rename journal decision

Default roadmap mengikuti keputusan YAGNI sebelumnya:

- Remove JSON filesystem journal for folder conflict rename.
- Keep in-process two-phase rename and best-effort rollback.
- Reject rename cycles/swaps whose destinations are not free if crash-safe recovery cannot be guaranteed without persistent evidence.

If product later requires crash-safe arbitrary swaps, that must be approved as an explicit exception; it cannot honestly be delivered without persistent recovery evidence.

## 4. Model and Agent Routing

| Work type                                     | Recommended role                | Model / reasoning                                              | Why                                           |
| --------------------------------------------- | ------------------------------- | -------------------------------------------------------------- | --------------------------------------------- |
| Locks, transaction boundaries, recovery state | `worker` with backend ownership | `gpt-5.6-sol`, `high` or `xhigh`                               | Highest correctness and concurrency risk      |
| Reconcile conflict/scoped projection          | `worker` with backend ownership | `gpt-5.6-sol`, `high`                                          | Cross-module invariants and failure semantics |
| Collection vertical slices                    | `worker`                        | `gpt-5.6-terra`, `high`; escalate to Sol for task finalization | Bounded business logic with existing patterns |
| Frontend React Query/UI consistency           | `frontend-developer`            | `gpt-5.6-terra`, `high`                                        | Frontend-specific state and mutation behavior |
| Performance measurement/scoped scan           | `performance-engineer`          | `gpt-5.6-terra`, `high`                                        | Must benchmark before optimizing              |
| Mechanical cleanup, bindings, dead code       | `worker`                        | `gpt-5.6-luna`, `medium`                                       | Low-risk, pattern-driven work                 |
| Checkpoint review                             | `reviewer`                      | `gpt-5.6-sol`, `high`                                          | Read-only correctness/regression gate         |

Do not assign two production-writing agents to the same ownership area concurrently. Test-only agents may work in parallel if their files do not overlap.

## 5. Standard Subtask Handoff Contract

Every subtask prompt should contain this preamble:

> You own only the files and responsibility listed in this task. You are not alone in the codebase. Do not revert, overwrite, or broadly format changes made by others. Re-read the current diff before editing and adapt to concurrent changes. Use `apply_patch` for edits and prefix shell commands with `rtk`. Implement only this task, run the listed focused verification, and return changed files, behavioral evidence, remaining risks, and exact commands/results.

Every subtask must return:

1. Summary of behavior changed.
2. Exact files changed.
3. Tests added or updated.
4. Verification commands and results.
5. Any assumption or residual risk.
6. No claim of full completion outside its assigned scope.

## 6. Dependency Graph

```text
P0 characterization
  |-- T0.1 concurrency/commit tests
  |-- T0.2 collection/recovery tests
  `-- T0.3 frontend consistency tests
          |
          v
P1 core boundary
  T1.1 lock coordinator -> T1.2 post-commit contract
          |
          +--------------------+
          v                    v
P2 collection correctness   P3 reconcile correctness
  T2.1 object apply           T3.1 conflict preservation
  T2.2 finalization           T3.2 scoped rename confirmation
  T2.3 exact rollback         T3.3 watcher suppression/session
  T2.4 atomic save            T3.4 source matching/error policy
          |                    |
          +----------+---------+
                     v
P4 single writer and performance
  T4.1 organizer writer removal
  T4.2 conflict path consolidation
  T4.3 true scoped discovery
  T4.4 one terminal reconcile / fast paths
                     |
                     v
P5 frontend/read model
  T5.1 cross-game queries
  T5.2 partial-success/progress refresh
  T5.3 runtime descriptor/read transaction
  T5.4 refresh mapping/bulk dedupe
                     |
                     v
P6 cleanup -> P7 regression/release gate
```

---

## Phase 0 - Characterization Before Production Changes

These three tasks are safe to run in parallel because they own separate test areas.

### T0.1 - Characterize lock inversion and post-commit failures

**Goal:** Add failing regression tests for the deadlock and projection-committed/runtime-effect-failed boundary.

**Agent:** `worker`, `gpt-5.6-sol`, `xhigh`.

**Ownership:** reconcile/orchestrator tests only; no production implementation.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/orchestrator/tests.rs`
- `src-tauri/src/services/disk_reconcile/source_recovery.rs` test module
- One test-only synchronization helper if necessary

**Acceptance criteria:**

- A deterministic barrier-based test reproduces reconcile/apply lock inversion and times out before the fix.
- A failure injected after projection commit proves the DB changed while source recovery attempts rollback.
- Tests do not rely on arbitrary sleeps.

**Verification:**

- `rtk cargo test --manifest-path src-tauri/Cargo.toml disk_reconcile::orchestrator`
- `rtk cargo test --manifest-path src-tauri/Cargo.toml source_recovery`

**Dependencies:** None.

**Size:** M.

### T0.2 - Characterize collection apply and recovery gaps

**Goal:** Add failing tests for parent object state, premature task completion, ambiguous rollback, and non-atomic replace.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** collection/apply/recovery tests only.

**Likely files:**

- `src-tauri/src/services/collection_service/tests/apply_tests.rs`
- `src-tauri/src/services/collection_service/tests/replace_tests.rs`
- `src-tauri/src/services/recovery_service.rs` test module
- `src-tauri/src/pipeline/tests/*` if already present

**Acceptance criteria:**

- A disabled parent object remains a failing case until object operations are implemented.
- Failure between apply and active-pointer update leaves a detectable pending recovery expectation.
- Clean, Modified, and Unsaved rollback expectations are distinct.
- Failure at each replace/finalization write demonstrates atomicity requirements.

**Verification:** focused collection and recovery Rust tests.

**Dependencies:** None.

**Size:** M.

### T0.3 - Characterize frontend stale and partial-success behavior

**Goal:** Add failing tests for game A data leaking into game B, partial move success without refresh, and terminal rename confirmation leaving progress active.

**Agent:** `frontend-developer`, `gpt-5.6-terra`, `high`.

**Ownership:** frontend tests only.

**Likely files:**

- `src/features/collections/hooks/useCollections.test.tsx`
- `src/features/collections/hooks/useCollectionRuntime.test.tsx`
- `src/features/mod-runtime/operations/sharedOperations.test.ts`
- `src/features/file-watcher/hooks.test.ts`

**Acceptance criteria:**

- Held response for game B cannot expose actionable game A collection/object data.
- One successful and one failed move must still expect a refresh.
- Any terminal reconcile result clears progress even when user action is still required.

**Verification:** `rtk pnpm test -- --run <focused files>`.

**Dependencies:** None.

**Size:** M.

### Checkpoint 0

- Characterization tests fail only for the audited gaps.
- A reviewer confirms the tests model real failure boundaries rather than implementation details.
- No production behavior has changed.

---

## Phase 1 - Concurrency and Commit Boundary

Production tasks in this phase are sequential.

### T1.1 - Establish one mutation lock order

**Goal:** Eliminate lock inversion using one acquisition helper and prevent pipeline steps from reacquiring locks.

**Agent:** `worker`, `gpt-5.6-sol`, `xhigh`.

**Ownership:** lock acquisition and collection/hotkey entry paths.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/orchestrator/entry.rs`
- `src-tauri/src/services/fs_utils/operation_lock.rs`
- `src-tauri/src/commands/collections/cmds.rs`
- `src-tauri/src/services/hotkeys/cycle_preset.rs`
- `src-tauri/src/pipeline/steps/batch_rename.rs`

**Implementation constraints:**

- Global invariant is `game_lock -> operation_lock`.
- Pass a lease/guards downward; do not acquire the same lock from pipeline steps.
- Do not introduce a generic lock framework.

**Acceptance criteria:**

- The T0.1 deadlock test passes repeatedly.
- Collection apply, hotkey cycle, and reconcile share the same acquisition order.
- No lock is held across unrelated frontend/event work.

**Verification:** targeted orchestrator, collection apply, and hotkey tests plus Clippy on touched targets.

**Dependencies:** T0.1.

**Size:** M.

### T1.2 - Make post-commit semantics explicit

**Goal:** Prevent a runtime-effect failure from masquerading as an uncommitted reconcile.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** reconcile result/finalization/source-recovery contract.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/orchestrator/run.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/state.rs`
- `src-tauri/src/services/disk_reconcile/source_recovery.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`
- Orchestrator/source recovery tests

**Implementation constraints:**

- After DB commit, return Applied plus structured warning/pending effects.
- Runtime effects remain idempotent and retryable.
- Source path rollback is legal only for pre-commit failure or SourceUnavailable.

**Acceptance criteria:**

- T0.1 post-commit failure test passes.
- Config path never rolls back after a successful projection commit.
- Pending effects are retried and acknowledged only after success.

**Dependencies:** T1.1.

**Size:** M.

### Checkpoint 1

- Run focused concurrency tests at least 50 iterations.
- Run source recovery tests.
- Read-only `reviewer` checks lock lifetime, error meaning, and cancellation paths.
- Do not start Phase 2 if any path can still acquire operation lock before game lock.

---

## Phase 2 - Collection Apply and Recovery Correctness

Tasks T2.1-T2.3 are sequential because they share apply/recovery contracts. T2.4 may run after T2.2 if it does not touch the same files.

### T2.1 - Apply parent object state

**Goal:** Make collection apply restore both terminal mod state and parent object state.

**Agent:** `worker`, `gpt-5.6-terra`, `high`; escalate review to Sol.

**Ownership:** apply planning and object/mod operation ordering.

**Likely files:**

- `src-tauri/src/pipeline/apply_pipeline.rs`
- `src-tauri/src/pipeline/steps/resolve_target.rs`
- `src-tauri/src/pipeline/steps/batch_rename.rs`
- Existing object toggle/mutation engine
- Collection apply tests

**Implementation constraints:**

- Build an explicit apply plan for terminal mods and parent objects.
- Apply child/mod operations before parent operations.
- Use one terminal reconcile; do not directly persist resulting status.

**Acceptance criteria:**

- Parent enabled/disabled snapshots apply correctly.
- Mixed parent/child raw states finish `Clean` against the target signature.
- Missing targets still follow existing `ignore_missing` behavior.

**Dependencies:** T1.1, T0.2.

**Size:** M.

### T2.2 - Finalize apply atomically

**Goal:** Keep recovery task pending until active baseline and task completion can be committed together.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** task lifecycle and active collection finalization.

**Likely files:**

- `src-tauri/src/pipeline/apply_pipeline.rs`
- `src-tauri/src/services/collection_service/apply.rs`
- `src-tauri/src/repo/task_repo.rs`
- `src-tauri/src/repo/collection_runtime_repo.rs`
- Apply/recovery tests

**Acceptance criteria:**

- Task cannot be `Completed` while active pointer still references the previous collection.
- Finalization failure leaves a recoverable pending task.
- Validation failure before mutation may end as Failed without creating false recovery work.

**Dependencies:** T2.1, T1.2.

**Size:** M.

### T2.3 - Persist exact rollback intent

**Goal:** Remove heuristic rollback selection.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** recovery task metadata and resolver.

**Likely files:**

- Canonical task migration/schema
- `src-tauri/src/domain/task.rs`
- `src-tauri/src/repo/task_repo.rs`
- `src-tauri/src/services/recovery_service.rs`
- Recovery tests

**Stored intent:**

- Target collection ID.
- Rollback snapshot/draft ID when Modified or Unsaved.
- Prior active baseline ID, nullable.
- Mutation-started/finalization state sufficient to distinguish safe retry from rollback.

**Acceptance criteria:**

- Clean A -> B rolls back to A.
- Modified A -> B restores the captured modified runtime, not plain A.
- Unsaved -> B restores Unsaved state.
- Deleted baseline draft remains restorable as Unsaved.

**Dependencies:** T2.2.

**Size:** M.

### T2.4 - Make save/replace collection one transaction

**Goal:** Atomically persist members, derived summary, active pointer, and Last Changes cleanup.

**Agent:** `worker`, `gpt-5.6-terra`, `high`.

**Ownership:** current-state save/replace transaction.

**Likely files:**

- `src-tauri/src/services/collection_service/current_state.rs`
- Collection projection persistence helper
- `src-tauri/src/repo/collection_runtime_repo.rs`
- Replace/save tests

**Acceptance criteria:**

- Failure injection at any write rolls back all writes.
- Successful save leaves runtime Clean and draft cleared in the same commit.
- Empty snapshot behavior is explicit and user-facing, not a generic validation surprise.

**Dependencies:** T2.2.

**Size:** M.

### Checkpoint 2

- Apply parent/object matrix passes.
- Clean/Modified/Unsaved retry and rollback matrix passes.
- Save/replace failure injection passes.
- Reviewer verifies task lifecycle and transaction boundaries.

---

## Phase 3 - Conflict, Watcher, and Source Correctness

T3.1 and T3.2 are sequential because both modify reconcile conflict handling. T3.3 and T3.4 may run in parallel after Checkpoint 1 if their file ownership remains separate.

### T3.1 - Preserve ambiguous conflict state

**Goal:** Stop lexicographic conflict representatives from changing enabled state or collection signature.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** conflict protection and projection writer behavior.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/mods.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/objects.rs`
- Runtime projection/signature helper if needed
- Conflict projection tests

**Acceptance criteria:**

- Existing DB row for a conflicted identity remains unchanged while unresolved.
- Unrelated roots still converge.
- A new identity with no prior row is represented as ambiguous/read-only and excluded from runtime signature.
- Grid conflict overlay continues to show every physical candidate.

**Dependencies:** T1.2.

**Size:** M.

### T3.2 - Scope rename confirmation

**Goal:** Prevent one ambiguous rename from blocking all projection or creating a global Cartesian candidate set.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** rename-confirmation grouping and reconcile protected scope.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/rename_confirmation.rs`
- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`
- Rename confirmation tests

**Acceptance criteria:**

- Ambiguity is grouped by nearest object/top-level scope.
- Only ambiguous keys are protected.
- Other roots are projected in the same run.
- Oversized candidate groups are capped/reported rather than expanded O(N*M).

**Dependencies:** T3.1.

**Size:** M.

### T3.3 - Fix scoped watcher suppression and ownership

**Goal:** Ensure app echoes remain harmless without losing genuine external changes.

**Agent:** `worker`, `gpt-5.6-sol`, `high`.

**Ownership:** watcher suppressor/session generation.

**Likely files:**

- `src-tauri/src/services/scanner/watcher/suppressor.rs`
- `src-tauri/src/services/scanner/watcher/mod.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/entry.rs`
- Watcher tests

**Acceptance criteria:**

- Scoped suppression ends with the mutation guard; no two-second blind tail.
- Queued app echoes may cause an idempotent reconcile but no duplicate error toast.
- External rename/delete immediately after app mutation is observed.
- Dirty/repaired generation is bound to watcher session and canonical root.

**Dependencies:** T1.1, T0.1.

**Size:** M.

### T3.4 - Harden source-directory matching and filesystem errors

**Goal:** Avoid treating partial overlap or changed enabled spelling as a previously reviewed matching library.

**Agent:** `worker`, `gpt-5.6-terra`, `high`.

**Ownership:** source inspection/fingerprint and strict reconcile classification policy.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/source_recovery.rs`
- `src-tauri/src/common/classifier.rs`
- Source recovery tests
- Classifier tests

**Acceptance criteria:**

- One matching filesystem identity is insufficient to auto-classify a mostly different library as Matching.
- Fingerprint includes raw relative spelling/status and object/mod kind.
- Relevant read/encoding failures abort before transaction with actionable error.
- Lenient classification remains available only for non-authoritative UI discovery if needed.

**Dependencies:** T1.2.

**Size:** M.

### Checkpoint 3

- Existing enabled/disabled DB state remains stable during unresolved conflict.
- Rename confirmation blocks only its scope.
- Immediate external edits after app mutations are observed.
- Source review cannot be reused after status/path changes.

---

## Phase 4 - Single Writer and Reconcile Performance

### T4.1 - Remove organizer direct projection writes

**Goal:** Make organizer operations disk-only and let reconcile own DB/reference transition.

**Agent:** `worker`, `gpt-5.6-terra`, `high`.

**Ownership:** organizer move/duplicate paths.

**Likely files:**

- `src-tauri/src/services/mods/organizer_move.rs`
- `src-tauri/src/services/mods/organizer_duplicates.rs`
- Organizer command entry
- Reconcile path-hint interface
- Organizer tests

**Acceptance criteria:**

- Organizer returns old/new path hints and does not update mod/object/collection rows directly.
- One reconcile transaction heals all references.
- Partial filesystem success is structured and still converges.

**Dependencies:** T3.1, T3.2.

**Size:** M.

### T4.2 - Consolidate conflict resolution and remove legacy path

**Goal:** Keep one conflict command/service/UI contract and eliminate duplicate DB writers.

**Agent:** `worker`, `gpt-5.6-terra`, `high`.

**Ownership:** backend conflict commands/services; frontend deletion may be a separate Luna subtask after contract settles.

**Likely files:**

- `src-tauri/src/commands/mods/conflict_cmds.rs`
- `src-tauri/src/services/mods/core_ops/conflict_resolution.rs`
- `src-tauri/src/services/mods/core_ops/folder_conflict_resolution.rs`
- Conflict command tests
- Command registration/bindings if signatures change

**Acceptance criteria:**

- Only the enhanced grouped conflict flow remains callable.
- Resolver mutates disk and returns rewrites; reconcile owns projection.
- No collection-healing error is swallowed.
- Apply the roadmap journal decision: remove JSON journal and reject unsupported cycles, unless product explicitly approves the exception.

**Dependencies:** T4.1.

**Size:** M; split frontend legacy deletion if it exceeds five files.

### T4.3 - Implement true scoped disk discovery

**Goal:** Stop every watcher/internal mutation from recursively classifying the entire library.

**Agent:** `performance-engineer`, `gpt-5.6-terra`, `high`.

**Ownership:** disk snapshot/scoped discovery only.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/common/classifier.rs`
- Snapshot/reconcile benchmark tests

**Design:**

1. Cheap global identity/name census for conflict detection.
2. Full metadata/INI classification only for changed roots.
3. Escalate once to full scan for startup, overflow, manual repair, root event, or cross-root ambiguity.

**Acceptance criteria:**

- A one-root watcher event classifies only that root plus cheap global census.
- Conflict detection remains complete.
- Full reconcile output is equivalent to scoped-plus-escalation output.
- All blocking filesystem work runs on the blocking pool.

**Dependencies:** Checkpoint 3, T4.1, T4.2.

**Size:** M.

### T4.4 - Reduce mutations to one terminal reconcile

**Goal:** Remove redundant full preflight/inline/full-terminal scans and add safe fast paths.

**Agent:** `performance-engineer`, `gpt-5.6-terra`, `high`.

**Ownership:** mutation preflight orchestration and scan-count tests.

**Likely files:**

- `src-tauri/src/services/disk_reconcile/emit.rs`
- Collection apply command/pipeline entry
- Mod command entries currently doing duplicate preflight
- Watcher event classification for thumbnail-only batches
- Performance tests

**Acceptance criteria:**

- Collection apply performs targeted preflight plus one terminal reconcile.
- Open Folder performs containment/conflict validation without full reconcile.
- Thumbnail-only watcher batches invalidate thumbnails without DB projection.
- Root-level watcher event forces full source validation.
- Before/after scan counts are recorded for large fixture.

**Dependencies:** T4.3, Checkpoint 2.

**Size:** M; split by command family if more than five production files overlap.

### Checkpoint 4

- Benchmark fixture records scan count and latency before/after.
- No conflict, missing, rename-healing, or startup regression.
- Reviewer checks that performance changes did not introduce another cache or stale snapshot authority.

---

## Phase 5 - Frontend Consistency and Read Model

T5.1 and T5.2 can run in parallel. T5.3 backend contract must land before its frontend binding consumer. T5.4 is cleanup after the earlier frontend tasks.

### T5.1 - Remove actionable cross-game placeholder data

**Goal:** Prevent game A data from appearing or being mutated while game B loads.

**Agent:** `frontend-developer`, `gpt-5.6-terra`, `high`.

**Ownership:** game-keyed collection/workspace query behavior.

**Likely files:**

- `src/features/collections/hooks/useCollections.ts`
- `src/features/collections/hooks/useCollectionRuntime.ts`
- `src/features/workspace-runtime/useWorkspaceViewModel.ts`
- Collection/workspace tests

**Acceptance criteria:**

- Changing game clears or non-actionably gates previous-game data.
- Collection/object actions cannot run on placeholder IDs.
- No regression to same-game optimistic rendering.

**Dependencies:** T0.3.

**Size:** M.

### T5.2 - Correct partial-success and terminal progress refresh

**Goal:** Make frontend refresh reflect successful backend effects even when the overall batch reports partial failure.

**Agent:** `frontend-developer`, `gpt-5.6-terra`, `high`.

**Ownership:** mod operation result application and reconcile progress.

**Likely files:**

- `src/features/mod-runtime/operations/sharedOperations.ts`
- `src/features/file-watcher/hooks.ts`
- `src/features/file-watcher/reconcileProgress.ts`
- Focused tests

**Acceptance criteria:**

- Successful rewrites are published before partial error is surfaced.
- Every terminal result clears progress.
- User-action-required state remains visible separately from running progress.

**Dependencies:** T0.3, T1.2.

**Size:** S-M.

### T5.3 - Split lightweight collection runtime descriptor

**Goal:** Avoid building/transferring full collection trees for topbar/global status reads and ensure a consistent DB snapshot.

**Agent:** backend `worker` plus frontend handoff; `gpt-5.6-terra`, `high`.

**Ownership part A:** backend read model.

**Likely backend files:**

- `src-tauri/src/services/collection_runtime_service.rs`
- `src-tauri/src/domain/runtime_state.rs`
- Collection live-state/query helpers
- Runtime service tests

**Backend acceptance criteria:**

- Runtime descriptor uses one read transaction.
- Descriptor contains active ID/name, status, missing count, safety summary, Last Changes, and counts only.
- Full preview/tree is a separate lazy query used by Collections page.
- Remove unused `current_mods`, `current_objects`, and redundant dirty fields from IPC.

**Ownership part B:** bindings and frontend consumers, assigned only after part A lands.

**Likely frontend files:** generated bindings, runtime hook, topbar, Collections page, tests.

**Dependencies:** Checkpoint 2, T5.1.

**Size:** Two M subtasks, backend then frontend.

### T5.4 - Collapse refresh mapping and bulk duplicate invalidation

**Goal:** Reduce `MutationClass -> Event -> Scope -> QueryKey` indirection without changing behavior.

**Agent:** `frontend-developer` or `worker`, `gpt-5.6-luna`, `medium`.

**Ownership:** frontend refresh mapping and bulk action refresh ownership.

**Likely files:**

- `src/features/workspace-runtime/optimistic/descriptorBuilders.ts`
- `src/features/runtime-sync/queryRefresh.ts`
- Bulk object/mod mutation hooks
- Refresh mapping tests

**Acceptance criteria:**

- One direct mapping to refresh scopes/query keys.
- Actual query keys are deduplicated, not only aliases.
- Bulk operation publishes once after the batch.
- No new event bus or cache layer.

**Dependencies:** T5.1, T5.2, T5.3.

**Size:** M.

### Checkpoint 5

- Game switch, partial success, rename confirmation, and collection topbar tests pass.
- Query Devtools/manual profiling shows no duplicate heavy runtime requests for topbar.
- Frontend reviewer verifies disabled/loading/placeholder accessibility states.

---

## Phase 6 - Schema and Dead-Code Simplification

This phase starts only after all behavioral checkpoints pass. Because backward DB compatibility is explicitly not required, canonical schema cleanup may reset development databases, but must be clearly documented.

### T6.1 - Remove dead runtime calculations and DTO fields

**Goal:** Delete calculations and contract fields with no production consumer.

**Agent:** `worker`, `gpt-5.6-luna`, `medium`.

**Likely files:**

- `src-tauri/src/services/app/runtime_effects.rs`
- `src-tauri/src/services/collection_service/current_state.rs`
- Disk reconcile result/domain DTO
- Generated bindings and focused tests

**Acceptance criteria:**

- Remove `handle_dirty_state` if only tests/dead caller remain.
- Remove unused `overlay_refresh_triggered` and redundant runtime fields after consumer audit.
- No behavior is replaced by another cache.

**Dependencies:** T5.3.

**Size:** S-M.

### T6.2 - Canonicalize collection schema

**Goal:** Remove corridor-era and duplicate derived persistence.

**Agent:** `worker`, `gpt-5.6-sol`, `high` for schema review; Luna may perform mechanical follow-up.

**Canonical storage target:**

- `collections`
- `collection_mods`
- `collection_objects`
- `collection_runtime_state`
- Recovery task metadata required by T2.3
- One signature and one count only if still used by measured read paths

**Candidates for removal after usage proof:**

- `is_last_unsaved`
- Corridor columns/indexes/tables
- `collection_nested_items`
- `collection_roots`
- Legacy signature tables
- Duplicate count/cache columns
- Stored-unsaved frontend branches

**Acceptance criteria:**

- Fresh DB migration creates only canonical schema.
- Repo code fails fast on missing required columns instead of compatibility fallbacks.
- Fresh-start, first collection, Modified, Unsaved, Last Changes, and missing-member tests pass.
- Database reset requirement is documented.

**Dependencies:** T2.3, T2.4, T6.1.

**Size:** Split into schema/backend and frontend cleanup M tasks.

### T6.3 - Remove inert queue and remaining legacy UI/code

**Goal:** Delete machinery that no longer owns behavior.

**Agent:** `worker`, `gpt-5.6-luna`, `medium`.

**Candidates:**

- Reconcile version/drain queue if enqueue remains behind the per-game mutex.
- Legacy pairwise conflict dialog/command bindings.
- Unreachable `collection.is_unsaved` branches.
- Duplicate folder/object query aliases without production consumers.
- Unused parameters and compatibility catches.

**Acceptance criteria:**

- Each deletion has a `git grep` usage proof.
- No broad renaming or speculative abstraction.
- Focused tests and build remain green after each deletion group.

**Dependencies:** T4.2, T5.4, T6.2.

**Size:** Several XS/S commits or subtasks, never one broad cleanup task.

### Checkpoint 6

- Fresh database boot and full collection lifecycle pass.
- Generated bindings and command allowlist are synchronized.
- Reviewer checks for dead branches, compatibility leftovers, and accidental behavior deletion.

---

## Phase 7 - Regression Matrix and Release Gate

### T7.1 - Complete convergence regression matrix

**Agent:** Multiple test-only workers may run in parallel by subsystem; `gpt-5.6-terra` or `luna` depending complexity.

**Required matrix:**

- In-app and external create/rename/move/delete/toggle.
- Flat mod, nested mod, parent object, child mod, enabled and disabled prefix.
- Single and bulk actions.
- Collection Clean/Modified/Unsaved/Last Changes/retry/rollback.
- Conflict rename and Trash resolution.
- App inactive/offline changes followed by startup.
- Source unavailable, empty, matching, and different folder.
- Watcher immediate post-mutation external change and overflow.
- Thumbnail-only event and thumbnail replacement.
- Auto-organize, category, info/thumbnail CRUD, archive extract/import.
- Game A -> B switch with held queries and rapid watcher replacement.

**Acceptance criteria:** every case asserts convergence between physical disk, core DB rows, runtime projection, collection runtime status, and visible frontend state where applicable.

**Dependencies:** Checkpoint 6.

**Size:** Separate M subtasks by subsystem.

### T7.2 - Full verification and final review

**Agent:** Primary coordinator runs commands; `reviewer` with `gpt-5.6-sol`, `high` performs read-only final review.

**Verification gates:**

- `rtk cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `rtk cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
- `rtk cargo test --manifest-path src-tauri/Cargo.toml`
- `rtk pnpm test -- --run`
- `rtk pnpm lint`
- `rtk pnpm i18n:lint`
- `rtk pnpm build`
- `rtk pnpm test:e2e` for relevant targeted specs before optional full E2E
- `rtk git diff --check`
- Command registration, permission, generated bindings, and docs audit

**Release-blocking review questions:**

- Can any two paths still acquire locks in opposite order?
- Can any command report failure after projection commit without indicating Applied?
- Can any disk mutation update projection outside reconcile?
- Can a conflict change enabled state before resolution?
- Can an apply task be Completed while active baseline is stale?
- Can game A data be actionable after switching to B?
- Does any targeted mutation still perform an unnecessary full recursive classification?

## 7. Safe Parallel Execution Waves

| Wave | Parallel tasks                                         | Sequential/coordination rule                                 |
| ---- | ------------------------------------------------------ | ------------------------------------------------------------ |
| A    | T0.1, T0.2, T0.3                                       | Tests only, separate ownership                               |
| B    | T1.1; T5.1 may run separately                          | T1.2 waits for T1.1                                          |
| C    | T2.1 -> T2.2 -> T2.3; T3.3 and T3.4 may run separately | No parallel edits to apply pipeline or orchestrator entry    |
| D    | T2.4 and T3.1                                          | T3.2 waits for T3.1                                          |
| E    | T4.1 -> T4.2 -> T4.3 -> T4.4                           | Reconcile/performance production changes sequential          |
| F    | T5.2, backend half T5.3                                | Frontend half T5.3 waits for bindings                        |
| G    | T5.4, T6.1                                             | Only after frontend behavior is stable                       |
| H    | T6.2 -> T6.3                                           | Cleanup sequential and evidence-driven                       |
| I    | T7.1 test slices                                       | Parallel by subsystem; final verification single coordinator |

## 8. Risk Register

| Risk                                                   | Impact                          | Mitigation                                                                         |
| ------------------------------------------------------ | ------------------------------- | ---------------------------------------------------------------------------------- |
| Dirty shared worktree                                  | User changes overwritten        | Explicit file ownership, re-read diff, `apply_patch`, no broad format              |
| Lock refactor introduces hidden reacquisition          | Deadlock remains                | Central lease plus barrier tests and reviewer lock graph                           |
| Post-commit warning is treated as success everywhere   | Side effects never repaired     | Structured pending effects and idempotent retry test                               |
| Object operations interact with child toggles          | Wrong final signature           | Child-first/parent-last plan and full state matrix                                 |
| Single-writer migration loses collection healing hints | Saved references become missing | Pass explicit old/new rewrite hints into reconcile transaction                     |
| Scoped scan misses cross-root conflict                 | Duplicate identity/SQL failure  | Cheap global identity census and escalation rule                                   |
| Performance changes add stale cache                    | New drift source                | No new cache; compare scoped result with full snapshot                             |
| Schema cleanup breaks existing local DB                | Startup failure                 | Explicit no-compatibility decision, fresh DB migration test, documented reset      |
| Removing conflict journal weakens swap crash recovery  | Staged folders after crash      | Reject unsupported cycles by default; require explicit product exception otherwise |
| Frontend cleanup invalidates too little                | Stale UI                        | Mapping parity tests before deleting old layer                                     |

## 9. Definition of Done

- All four release blockers from the audit have regression tests and fixes.
- Disk remains authoritative and projection has one writer.
- Lock ordering is uniform and mechanically enforced by ownership/API shape.
- Collection parent states, active baseline, task status, draft, and rollback intent remain consistent across success, failure, and restart.
- Conflict and rename ambiguity block only affected identities/scopes.
- External changes during or immediately after app mutation are eventually observed.
- Targeted mutations avoid full recursive classification unless escalation conditions apply.
- Frontend never exposes actionable previous-game data and refreshes partial successes.
- Legacy corridor/stored-unsaved/dead refresh code is removed only after behavior parity tests.
- No new dependency, persistent cache, polling daemon, generic saga, or event bus.
- Targeted tests, full Rust/frontend suites, lint, i18n lint, build, E2E, diff check, and final read-only review pass.
