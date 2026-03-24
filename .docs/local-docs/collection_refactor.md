# Plan: Simplify Collections & Privacy Systems for Stability

TL;DR
Your collections system has solid architecture but suffers from complex workspace reconciliation, heuristic signature matching with edge cases, and ambiguous state ownership (FS vs DB vs Cache). The privacy system is missing visual masking and has a PIN lockout persistence bug. This plan restructures both systems around KISS principles: explicit state ownership, deterministic identity matching, and clear error boundaries.

---

## Identified Gaps & Issues

### Collections System

1. Workspace reconciliation is convoluted — nested fallback logic in workspaceSelection.ts with 3 row kinds; easy to break on edge cases
2. Signature matching relies on heuristics — breaks if mods renamed on disk; doesn't use immutable mod IDs
3. FS↔DB sync is implicit — mod enabled/disabled state tracked via DISABLED prefix; info.json updates fire OUTSIDE transactions (race condition risk)
4. State scattered across layers — Zustand + TanStack Query + DB cache; no single source of truth
5. No early validation — safe-mode context checked AFTER expensive preflight ops
6. No rollback on failures — partial apply failures leave state inconsistent

### Privacy/Safe-Mode System

- ✗ Visual masking NOT implemented ([Hidden Mod] + blur missing when Safe Mode active)
- ✗ PIN lockout state stored in-memory; resets on app restart
- ✗ Bulk corridor-switch warnings not surfaced to user
- ✗ No validation on sub-mod filtering depth

---

## Implementation Plan

### Phase 1: Collections System Refactoring (Main Focus)

#### 1A. Simplify Workspace State (Frontend)

Remove nested fallback logic. Extract into pure function: resolveActiveCollection(runtime, savedCollections, prefs) → single result or error.

- Flatten Zustand: workspaceSelection.gameId + collectionId + state (enum)
- Consolidate preferences in one store (no localStorage scatter)
- Outcome: No ambiguous fallbacks; single decision path

#### 1B. Unify State Ownership (Backend)

Make explicit: Mod enabled/disabled ↔ FS + DB ALWAYS synchronized.

- Add mods.disabled_reason column (null | 'SYSTEM' | 'USER')
- mods.status becomes computed from FS state
- Verify FS↔DB consistency at operation boundaries (spot-check 10 newest mods before ops)
- Move info.json updates into DB transactions (use watcher suppression)
- Outcome: FS and DB never diverge; no race conditions

#### 1C. Simplify Signature Matching (Backend)

Replace heuristics with deterministic identity.

- Use immutable mod IDs (SHA1 of relative path, already in rules)
- Signature = ordered list of (mod_id, object_state) pairs (not paths)
- Implement 3-level match: exact, prefix, or no-match
- Cache signature in DB to avoid rescans
- Outcome: Renaming mods on disk doesn't break collection identity; no edge cases

#### 1D. Refactor Workspace Reconciliation (Backend + Frontend)

Single command resolves "what collection should be active?"

- New resolveWorkspaceContext(game_id, safe_mode) command
- Returns: WorkspaceContext { current_signature, matched_collection_id?, unsaved_snapshot_id?, recommendations }
- Backend handles all logic (no fallback chains in FE)
- Frontend uses result to populate UI (no ambiguous states)
- Outcome: Same FS always resolves to same recommendation; user understands why a collection is active

#### 1E. Early Safe-Mode Context Validation (Backend)

Fail before expensive operations.

- Before preflight: Check collection.is_safe_context === current_safe_mode
- Auto-detect context from member mods on create/update
- Outcome: Wrong-context collections rejected immediately

#### 1F. Test Suite for Workspace Logic (Rust + TypeScript)

Prevent regressions.

- Rust: signature matching, mod identity, safe-mode validation, apply→active flow
- TypeScript: workspace selection, state sync, masking
- Outcome: Future changes covered by safety net

### Phase 2: Privacy/Safe-Mode Gap Fixes (Quick Wins)

#### 2A. Visual Masking: Implement [Hidden Mod] + Blur

- Add conditional rendering in FolderCard + ObjectList
- When isSafeMode && !mod.is_safe: Show <span className="blur-[12px]">[Hidden Mod]</span>

#### 2B. PIN Lockout Persistence

- Remove in-memory state; always read/write from DB
- Lockout state survives app restart

#### 2C. Bulk-Switch Warning Surface

- Track detailed failures in CorridorSwitchResult
- Toast shows: "3 mods couldn't be disabled" + expandable list

---

## Critical Files to Modify

| File                               | Change                                                   |
| ---------------------------------- | -------------------------------------------------------- |
| workspaceSelection.ts              | Replace nested logic with pure resolveActiveCollection() |
| useAppStore.ts                     | Flatten workspace structure                              |
| queryKeys.ts                       | Add resolveWorkspaceContext query                        |
| storage.rs                         | Move workspace logic into service                        |
| runtime_snapshot.rs                | Use mod IDs instead of heuristics                        |
| collection_cmds.rs                 | Add early safe-mode validation, new resolver command     |
| src-tauri/src/database/migrations/ | Add mods.disabled_reason column                          |
| src/components/FolderCard.tsx      | Implement visual masking                                 |
| pin_guard.rs                       | Remove in-memory lockout state                           |

---

## Parallelizable Work Batches

### Batch 1 (Parallel) — Foundation

- 1B: State ownership (FS↔DB sync)
- 1A: Frontend state flattening
- 2A, 2B: Visual masking + PIN lockout (quick wins)

### Batch 2 — Depends on Batch 1

- 1C: Signature matching (uses mod IDs from 1B)
- 1E: Safe-mode validation (uses mod tracking from 1B)

### Batch 3 — Depends on Batch 2

- 1D: Workspace reconciliation (uses simplified state + new signatures)

### Batch 4 — Final

- 1F: Test suite

## Verification Steps

### Automated:

- ✓ Rust tests: signature identity, safe-mode validation, apply→active flow
- ✓ TypeScript tests: workspace selection, state sync, visual masking
- ✓ ESLint + clippy

### Manual E2E:

- Create collection in Safe Mode → Apply → Workspace shows it active
- Restart app → Same workspace state auto-resolves
- Rename mod on disk → Rescan → Collection identity preserved
- Apply wrong-context collection → Rejected before preflight
- Safe Mode toggle → NSFW mods show [Hidden Mod] + blur
- PIN lockout → Persists after app restart

## Scope Boundaries

Included: Workspace reconciliation, mod status tracking, signature matching, safe-mode validation, visual masking, PIN persistence, tests
Excluded: UI refactoring beyond state mgmt, new collection features, dedup integration, disk watcher optimization

## Key Decisions

1. Mod ID as Identity: Immutable mod IDs (SHA1 of path) replace heuristics → eliminates edge cases, requires migration
2. Early Validation: Check safe-mode context before preflight → fail fast
3. Explicit State Ownership: Each layer (FS/DB/Cache) has clear responsibility
4. No Breaking Changes: New resolveWorkspaceContext is additive; phased rollout possible
5. Test-First Refactoring: Write tests BEFORE changes; maintain safety net

---

## Implementation Progress (March 20, 2026)

### Privacy/Safe-Mode: Batch Started

- ✅ Added backend no-op guard in `set_safe_mode_enabled` to skip redundant corridor switch execution when mode is unchanged
- ✅ Hardened `useSafeModeToggle` prechecks to fail fast when no active game is selected before preview/switch
- ✅ Improved warning surface in mode-switch toast to include concise warning details (first items + remaining count)
- ✅ Simplified corridor sub-mod depth filtering in privacy service to remove ambiguous fallback path handling
- ✅ Made corridor switch warning behavior explicit by stage (`disable-stage` / `restore-stage`) and propagated to command result
- ✅ Added hook-level tests for warning visibility and no-active-game precheck in `useSafeModeToggle.test.tsx`
- ✅ Refactored hook modal choreography with shared `openConfirmForTarget()` path to remove duplicated preview-open logic
- ✅ Strengthened store hygiene test to verify post-switch selection/path reset in `useAppStore.test.ts`
- ✅ Added backend regression test for stage-prefixed warning contract in privacy service tests
- ✅ Targeted frontend safe-mode tests passing:
  - `ModeSwitchConfirmModal.test.tsx` (4/4)
  - `PinEntryModal.test.tsx` (2/2)
- ✅ Focused backend command test filter passing for settings command module
- ✅ Focused privacy depth test passing: `test_switch_mode_preserves_depth_1`
- ✅ New warning regression passing: `test_switch_mode_returns_stage_prefixed_restore_warnings`
- ✅ Expanded safe-mode frontend suite passing:
  - `useSafeModeToggle.test.tsx` (3/3)
  - `ModeSwitchConfirmModal.test.tsx` (4/4)
  - `PinEntryModal.test.tsx` (2/2)
  - `useAppStore.test.ts` (17/17)

### Next Implementation Delta

1. Continue backend privacy simplification in `services/privacy/mod.rs` (transaction boundary clarity and deterministic warning propagation under partial restore)
2. Add/verify cache consistency assertions around the single invalidation path after mode switch
3. Expand command-level tests for no-op switch guard behavior in settings command layer
