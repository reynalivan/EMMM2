# Corridor Runtime Stability & Simplification Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make collection runtime and safe/unsafe corridor switching deterministic by centralizing runtime transition orchestration, reducing duplicate invalidation/refetch logic, and enforcing strict runtime vs UI workspace boundaries.

**Architecture:** We will keep the existing product behavior but simplify control flow around one corridor identity `(game_id, safe_mode)` and one transition orchestrator. Frontend will route apply/save/switch/manual-drift events through a single runtime flow, while backend remains authoritative for strict runtime snapshots and parity checks. Regression protection will be driven by transition-matrix tests and event-ordering tests.

**Tech Stack:** React 19, TypeScript, TanStack Query, Zustand, Vitest, Rust, SQLx, Tauri v2

---

## Implementation Context (Read Before Task 1)

- Filesystem is source of truth; runtime cache must converge to strict backend state.
- Corridor identity is always `(game_id, safe_mode)`.
- Do not change user-visible features in this plan; this is stabilization/simplification only.
- Maintain DRY and YAGNI: remove duplicate orchestration, avoid adding new abstractions unless they replace existing duplication.

## File Responsibility Map

### Frontend runtime orchestration

- Modify: `src/features/collections/queryKeys.ts`
  - Responsibility: single canonical query key builder for corridor runtime and preview paths.
- Modify: `src/features/collections/utils/refetchCollectionRuntime.ts`
  - Responsibility: orchestrated strict runtime refetch entrypoint for collection-triggered transitions.
- Modify: `src/features/collections/utils/refetchCurrentCorridorRuntime.ts`
  - Responsibility: current corridor strict refetch adapter (must become thin wrapper or merge target).
- Modify: `src/features/collections/utils/invalidateCorridorRuntime.ts`
  - Responsibility: corridor runtime invalidation policy.
- Modify: `src/features/collections/utils/invalidateCollectionRuntime.ts`
  - Responsibility: collection runtime invalidation policy (remove overlap with corridor invalidation).
- Modify: `src/features/collections/hooks/useCollections.ts`
  - Responsibility: transition dispatch for apply/save/manual-drift event paths.
- Modify: `src/features/collections/components/ApplyCollectionModal.tsx`
  - Responsibility: apply preview + post-apply transition integration.
- Modify: `src/features/collections/components/SaveCollectionModal.tsx`
  - Responsibility: save snapshot transition integration.
- Modify: `src/hooks/useSafeModeToggle.ts`
  - Responsibility: safe/unsafe switch transition sequencing and strict runtime freshness gating.

### Frontend state boundary

- Modify: `src/stores/useAppStore.ts`
  - Responsibility: strict runtime state vs UI workspace selection boundary.
- Modify: `src/lib/corridorSelection.ts`
  - Responsibility: deterministic selection derivation from strict runtime + workspace intent.

### Backend strict runtime parity

- Modify: `src-tauri/src/services/corridor_runtime.rs`
  - Responsibility: strict runtime contract and corridor snapshot consistency.
- Modify: `src-tauri/src/database/corridor_runtime_cache_repo.rs`
  - Responsibility: runtime cache fetch/update consistency for corridor key.
- Modify: `src-tauri/src/services/privacy/tests/privacy_service_tests.rs`
  - Responsibility: safe/unsafe switch parity regression coverage.
- Modify: `src-tauri/tests/collections_service.rs`
  - Responsibility: collection apply/save strict runtime convergence checks.
- Modify: `src-tauri/tests/collection_runtime_materialization.rs`
  - Responsibility: runtime materialization and named/unsaved transition regression coverage.

### Frontend tests

- Modify: `src/features/collections/hooks/useCollections.test.ts`
- Modify: `src/features/collections/components/ApplyCollectionModal.test.tsx`
- Modify: `src/features/collections/utils/reconcileActiveCollection.test.ts`
- Create: `src/features/collections/utils/corridorRuntimeTransition.test.ts`
- Create: `src/features/collections/utils/corridorPreviewParity.test.ts`

---

## Chunk 1: Worktree + Transition Contract Baseline

### Task 1: Create isolated worktree and verify baseline

**Files:**

- Modify: none
- Test: none

- [ ] **Step 1: Create feature worktree**
  - Use `@superpowers/using-git-worktrees` workflow.
  - Create branch: `fix/corridor-runtime-stability`.

- [ ] **Step 2: Verify branch and clean status**
  - Run: `git status`
  - Expected: clean working tree on new branch.

- [ ] **Step 3: Capture baseline test state (frontend targeted)**
  - Run: `pnpm test src/features/collections/hooks/useCollections.test.ts`
  - Expected: command succeeds; capture any existing unrelated failures.

- [ ] **Step 4: Capture baseline test state (backend targeted)**
  - Run: `cargo test --test collections_service`
  - Expected: command succeeds; capture any existing unrelated failures.

- [ ] **Step 5: Commit baseline notes if repo convention requires tracking**
  - If no file changes, skip commit.

### Task 2: Lock corridor runtime transition contract with failing tests

**Files:**

- Create: `src/features/collections/utils/corridorRuntimeTransition.test.ts`
- Modify: `src/features/collections/hooks/useCollections.test.ts`
- Modify: `src/features/collections/components/ApplyCollectionModal.test.tsx`

- [ ] **Step 1: Add failing test for apply transition sequence**
  - Assert event order: optimistic update (if any) -> strict refetch -> reconcile.

- [ ] **Step 2: Add failing test for manual-drift transition**
  - Assert named snapshot transitions to unsaved runtime deterministically.

- [ ] **Step 3: Add failing test for save transition**
  - Assert unsaved runtime transitions back to named snapshot after save success.

- [ ] **Step 4: Run targeted frontend tests to verify failures**
  - Run: `pnpm test src/features/collections/utils/corridorRuntimeTransition.test.ts`
  - Expected: FAIL with missing/incorrect transition orchestration.

- [ ] **Step 5: Commit tests-only checkpoint**
  - Run: `git add <new/modified test files>`
  - Run: `git commit -m "test: add corridor runtime transition contract coverage"`

---

## Chunk 2: Frontend Orchestration Consolidation

### Task 3: Unify query keys and runtime refetch orchestration

**Files:**

- Modify: `src/features/collections/queryKeys.ts`
- Modify: `src/features/collections/utils/refetchCollectionRuntime.ts`
- Modify: `src/features/collections/utils/refetchCurrentCorridorRuntime.ts`
- Modify: `src/features/collections/utils/invalidateCorridorRuntime.ts`
- Modify: `src/features/collections/utils/invalidateCollectionRuntime.ts`
- Test: `src/features/collections/utils/corridorRuntimeTransition.test.ts`

- [ ] **Step 1: Implement canonical corridor runtime query key builder**
- [ ] **Step 2: Route refetch helpers through one orchestrated entrypoint**
- [ ] **Step 3: Remove duplicate invalidation behavior and keep one policy path**
- [ ] **Step 4: Run targeted tests**
  - Run: `pnpm test src/features/collections/utils/corridorRuntimeTransition.test.ts`
  - Expected: previously failing transition/refetch tests now PASS.

- [ ] **Step 5: Commit orchestration consolidation**
  - Run: `git add src/features/collections/queryKeys.ts src/features/collections/utils/refetchCollectionRuntime.ts src/features/collections/utils/refetchCurrentCorridorRuntime.ts src/features/collections/utils/invalidateCorridorRuntime.ts src/features/collections/utils/invalidateCollectionRuntime.ts`
  - Run: `git commit -m "refactor: consolidate corridor runtime refetch and invalidation"`

### Task 4: Enforce strict runtime vs UI workspace boundaries

**Files:**

- Modify: `src/stores/useAppStore.ts`
- Modify: `src/lib/corridorSelection.ts`
- Modify: `src/features/collections/hooks/useCollections.ts`
- Modify: `src/features/collections/utils/reconcileActiveCollection.test.ts`
- Test: `src/features/collections/hooks/useCollections.test.ts`

- [ ] **Step 1: Add failing boundary tests (strict active cannot be overwritten by UI selection intent)**
- [ ] **Step 2: Implement state separation in store/selectors and hook usage**
- [ ] **Step 3: Remove ambiguous fallback behavior that mixes strict and UI state**
- [ ] **Step 4: Run targeted tests**
  - Run: `pnpm test src/features/collections/hooks/useCollections.test.ts`
  - Run: `pnpm test src/features/collections/utils/reconcileActiveCollection.test.ts`
  - Expected: PASS.

- [ ] **Step 5: Commit boundary enforcement**
  - Run: `git add src/stores/useAppStore.ts src/lib/corridorSelection.ts src/features/collections/hooks/useCollections.ts src/features/collections/utils/reconcileActiveCollection.test.ts src/features/collections/hooks/useCollections.test.ts`
  - Run: `git commit -m "fix: enforce strict runtime and workspace selection boundaries"`

---

## Chunk 3: Preview Parity and Safe/Unsafe Switch Stability

### Task 5: Unify preview derivation for apply/save/switch

**Files:**

- Create: `src/features/collections/utils/corridorPreviewParity.test.ts`
- Modify: `src/features/collections/components/ApplyCollectionModal.tsx`
- Modify: `src/features/collections/components/SaveCollectionModal.tsx`
- Modify: `src/hooks/useSafeModeToggle.ts`
- Test: `src/features/collections/components/ApplyCollectionModal.test.tsx`

- [ ] **Step 1: Add failing parity tests for leaving-side and target-side preview consistency**
- [ ] **Step 2: Route apply/save preview logic through one derivation path**
- [ ] **Step 3: Route safe/unsafe switch confirmation preview through same derivation contract**
- [ ] **Step 4: Ensure switch flow waits for required strict runtime freshness before confirming UI state**
- [ ] **Step 5: Run targeted tests**
  - Run: `pnpm test src/features/collections/utils/corridorPreviewParity.test.ts`
  - Run: `pnpm test src/features/collections/components/ApplyCollectionModal.test.tsx`
  - Expected: PASS.

- [ ] **Step 6: Commit preview parity stabilization**
  - Run: `git add src/features/collections/utils/corridorPreviewParity.test.ts src/features/collections/components/ApplyCollectionModal.tsx src/features/collections/components/SaveCollectionModal.tsx src/hooks/useSafeModeToggle.ts src/features/collections/components/ApplyCollectionModal.test.tsx`
  - Run: `git commit -m "fix: unify corridor preview derivation across apply save and switch"`

---

## Chunk 4: Backend Parity + Regression Matrix

### Task 6: Harden backend strict runtime parity contract

**Files:**

- Modify: `src-tauri/src/services/corridor_runtime.rs`
- Modify: `src-tauri/src/database/corridor_runtime_cache_repo.rs`
- Modify: `src-tauri/tests/collection_runtime_materialization.rs`
- Modify: `src-tauri/tests/collections_service.rs`

- [ ] **Step 1: Add failing tests for named/unsaved and strict runtime materialization convergence**
- [ ] **Step 2: Implement minimal backend fixes to satisfy strict corridor key contract**
- [ ] **Step 3: Run targeted backend tests**
  - Run: `cargo test --test collection_runtime_materialization`
  - Run: `cargo test --test collections_service`
  - Expected: PASS.

- [ ] **Step 4: Commit backend parity fixes**
  - Run: `git add src-tauri/src/services/corridor_runtime.rs src-tauri/src/database/corridor_runtime_cache_repo.rs src-tauri/tests/collection_runtime_materialization.rs src-tauri/tests/collections_service.rs`
  - Run: `git commit -m "fix: stabilize backend corridor runtime parity"`

### Task 7: Add safe/unsafe privacy switch regressions in Rust tests

**Files:**

- Modify: `src-tauri/src/services/privacy/tests/privacy_service_tests.rs`

- [ ] **Step 1: Add failing tests for safe->unsafe->safe chain parity expectations**
- [ ] **Step 2: Add test for manual drift reconciliation during corridor switch flow**
- [ ] **Step 3: Implement minimal privacy service adjustments if required by tests**
- [ ] **Step 4: Run privacy test suite**
  - Run: `cargo test privacy_service_tests`
  - Expected: PASS.

- [ ] **Step 5: Commit privacy parity regressions and fixes**
  - Run: `git add src-tauri/src/services/privacy/tests/privacy_service_tests.rs`
  - Run: `git commit -m "test: add corridor privacy switch parity regressions"`

---

## Chunk 5: Final Verification and Documentation

### Task 8: Full targeted verification and cleanup

**Files:**

- Modify: `docs/superpowers/plans/2026-03-19-corridor-runtime-stability-simplification.md` (checklist status only, optional)

- [ ] **Step 1: Run frontend verification bundle**
  - Run: `pnpm test src/features/collections`
  - Expected: PASS.

- [ ] **Step 2: Run backend verification bundle**
  - Run: `cargo test --test collections_service --test collection_runtime_materialization`
  - Expected: PASS.

- [ ] **Step 3: Run lint/format checks relevant to modified areas**
  - Run: `pnpm lint`
  - Run: `cargo fmt -- --check`
  - Run: `cargo clippy --all-targets --all-features`
  - Expected: PASS or pre-existing issues documented.

- [ ] **Step 4: Run git status and ensure only intended files changed**
  - Run: `git status`
  - Expected: clean or only intentional staged changes.

- [ ] **Step 5: Final commit for any remaining non-functional cleanup**
  - Run: `git add <remaining intended files>`
  - Run: `git commit -m "chore: finalize corridor runtime stability refactor verification"`

---

## Acceptance Criteria

- Deterministic transition behavior for apply, save, manual drift, and safe/unsafe switch.
- Single canonical query-key and refetch/invalidation orchestration path for corridor runtime.
- No UI-selection path can silently override strict runtime active state.
- Preview parity holds for apply/save/switch scenarios in both frontend and backend regression tests.
- Targeted FE and BE suites pass with no new flaky behavior.

## Non-Goals

- No new user-facing collection features.
- No broad architecture rewrite outside corridor runtime/selection path.
- No unrelated store or hook refactors.

## Execution Notes for Agentic Workers

- Use `@superpowers/subagent-driven-development` when subagents are available.
- If subagents are unavailable, use `@superpowers/executing-plans` in strict task order.
- Keep commits small and scoped to one task.
- If blocked by unclear behavior, stop and ask before proceeding.
