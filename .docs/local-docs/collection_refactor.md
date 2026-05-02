# Plan: Simplify Collections & Privacy Systems for Stability

TL;DR
The collections and privacy systems are now centered on one small runtime mutation engine for `DISABLED ` prefix changes. Filesystem remains the physical truth, DB `mods` is the synchronized projection, collections are saved snapshots, and corridor state owns the active/unsaved pointers. Keep future work focused on deterministic identity, clear error boundaries, and avoiding a generic orchestration layer.

## Current Stabilization Status (May 2026)

- ✅ Collection apply and Safe/Unsafe switch now share `runtime_mutation_engine` for filesystem rename + DB projection.
- ✅ Missing collection members return `MissingMods` before mutation unless `ignore_missing = true`; skipped members become warnings.
- ✅ Safe/Unsafe switch returns backend-authoritative `active_safe`, restored collection, and staged warnings.
- ✅ Safe Mode persistence is backend authoritative through `safe_mode.enabled`.
- ✅ PIN failed attempts and lockout are DB-backed, so restart does not clear lockout.
- ✅ Folder card leak guard masks out-of-corridor mod names with `[Hidden Mod]`/blur fallback.
- ⚠️ Remaining concern: workspace reconciliation/signature matching can still be simplified further, but it should evolve around projected state and stable IDs rather than another frontend fallback chain.

---

## Identified Gaps & Issues

### Collections System

1. Workspace reconciliation is still more complex than necessary — keep pressure toward one backend-resolved active/unsaved corridor state.
2. Signature matching should prefer stable `mod_id` and canonical `folder_path_key`; avoid adding more path-string heuristics.
3. FS↔DB sync for enabled/disabled state is now explicit through the runtime mutation engine; future manual toggles should use the same boundary.
4. State scattered across layers — Zustand + TanStack Query + DB cache; no single source of truth
5. Early validation exists for collection corridor and missing mods; keep apply/switch validation before disk mutation.
6. Runtime mutation rollback is best-effort; tests should keep covering DB failure after successful FS rename.

### Privacy/Safe-Mode System

- ✓ Visual masking implemented as a leak guard in folder cards.
- ✓ PIN lockout persisted in DB-backed `pin_config`.
- ✓ Bulk corridor-switch warnings are surfaced in switch result and frontend toast.
- ✓ Corridor switch depth filtering is explicit enough to avoid top-level Object mutation.

---

## Implementation Plan

### Phase 1: Collections System Refactoring (Main Focus)

#### 1A. Simplify Workspace State (Frontend)

Remove nested fallback logic. Extract into pure function: resolveActiveCollection(runtime, savedCollections, prefs) → single result or error.

- Flatten Zustand: workspaceSelection.gameId + collectionId + state (enum)
- Consolidate preferences in one store (no localStorage scatter)
- Outcome: No ambiguous fallbacks; single decision path

#### 1B. Unify State Ownership (Backend) — Implemented for Enabled/Disabled Mutation

Make explicit: Mod enabled/disabled ↔ FS + DB ALWAYS synchronized.

- Use `mods.disabled_reason` consistently (`NULL`, `SYSTEM`, `COLLECTION`, user/manual reasons).
- Keep `mods.status` and `folder_path_key` as DB projection synchronized by the runtime mutation engine.
- Verify FS↔DB consistency at operation boundaries (spot-check 10 newest mods before ops)
- Move info.json updates into DB transactions (use watcher suppression)
- Outcome: Apply/switch share one mutation path; future toggle work should join the same engine instead of custom rename logic.

#### 1C. Simplify Signature Matching (Backend)

Replace heuristics with deterministic identity.

- Use stable `mod_id` first, with canonical `folder_path_key` as the healing fallback.
- Signature = ordered list of (`mod_id`, `folder_path_key`, object_state) pairs, not raw display paths.
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

#### 2A. Visual Masking: Implement [Hidden Mod] + Blur — Implemented

- Add conditional rendering in FolderCard + ObjectList
- When isSafeMode && !mod.is_safe: Show <span className="blur-[12px]">[Hidden Mod]</span>

#### 2B. PIN Lockout Persistence — Implemented

- Remove in-memory state from service behavior; always read/write failed attempts and lockout from DB.
- Lockout state survives app restart

#### 2C. Bulk-Switch Warning Surface — Implemented

- Track staged warnings in `SwitchResult.warnings`.
- Toast summarizes warning count; detailed warning expansion can be added later only if users need it.

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
| pin_service.rs / pin_repo.rs       | Persist failed attempts and lockout state in DB          |

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

1. Mod identity first: prefer stable `mod_id` and canonical `folder_path_key`; avoid raw display path heuristics.
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

1. Replace remaining manual toggle rename paths with `runtime_mutation_engine` if any still bypass it.
2. Keep cache consistency assertions around the corridor-explicit collection query key and backend mode result.
3. Expand command-level tests for no-op switch guard behavior in the collection command layer.
