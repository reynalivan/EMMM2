# Implementation Plan: Onboarding Index Progress, Stable Grid Layout, and Scoped Folder-Conflict Reconcile

## Outcome

Improve three user-visible flows without changing the disk-first ownership model:

1. First-time onboarding shows real indexing progress and a bounded, honest estimate.
2. The Mods grid stays sharp and correctly positioned after maximize/fullscreen and resize.
3. Folder-name conflicts become a resolvable queue and quarantine only the affected filesystem scope; unrelated mods continue to reconcile and remain actionable.

Filesystem remains the source of truth. The database, runtime projection, React Query cache, and interface are projections of the latest safe filesystem snapshot. No filesystem journal, polling daemon, new event bus, or generic saga abstraction is introduced.

## Audit Findings

- `WelcomeScreen.handleFinalize` saves games and then calls `reconcileDiskStateCmd` sequentially, but the reconcile contract has no progress payload. The current indexing view is therefore necessarily an indeterminate spinner.
- `useFolderGridLayout` begins at a fixed `containerWidth = 800`, observes only asynchronously through `ResizeObserver`, and virtualizes rows using a cached estimated height and `transform: translateY(...)`. `FolderGrid` and the scroll owner also lack explicit `min-h-0/min-w-0` constraints. This is a credible layout/cache race when the desktop window changes size; it must be characterized before choosing the smallest fix.
- `FolderConflictManager` already replaces the store with the post-rename/post-trash reconcile result and selects the first remaining group when the selected group disappears. It does not retain a completed-step history, calculate resolved/remaining counts, or deliberately select the next group relative to the completed one.
- `reconcile_disk_projection` currently detects any conflict and returns before the projection writer. `applyDiskReconcileResult` returns early for that status, and `useFolderGridViewModel` disables every mutation when any folder conflict exists. `ensure_mutation_preflight` similarly rejects every directory mutation for the game.

## Recommended Design: Scoped Quarantine, Not Global Blocking

Use a read-only full-snapshot preflight to determine the complete conflict report, then partition the snapshot into:

- **protected conflict scopes:** each conflicting folder candidate and every descendant when the conflict is an object/parent folder;
- **safe scope:** every remaining physical path.

The projection writer applies only the safe scope in its usual transaction. It must neither upsert, rename, prune, rewrite collection references, nor accept watcher rename hints for a protected scope. Existing rows for protected paths remain the last valid snapshot until the conflict is resolved. The runtime projection is rebuilt from that mixed-but-valid database state, so safe paths converge while affected cards can be marked "needs resolution".

Add a terminal `AppliedWithFolderConflicts` reconcile status (rather than overloading `Applied` or retaining global `BlockedByFolderConflicts`). Its result carries both the fresh safe-scope changes and the authoritative conflict report. `BlockedByFolderConflicts` may be removed once all callers move, because this repository does not require wire compatibility with older versions.

Mutation authorization becomes target-aware:

- Single-item actions are blocked only when their target is inside a protected scope.
- Bulk actions execute the safe subset and return structured `blocked_paths` for the skipped subset; the UI shows the exact count/reason.
- Planned actions such as collection apply, auto-organize, and import determine their affected paths before writing. They proceed only for a disjoint plan; a plan touching a protected scope is rejected before its write phase.
- Truly root-wide actions whose affected paths cannot be known in advance remain guarded with an explicit reason. This is the narrow safety exception, not the default.

This preserves the prior atomicity guarantee where it matters: no partial DB mutation inside a protected identity group. It is simpler and safer than inventing per-row conflict recovery or guessing which duplicate folder wins.

## Contract Changes

### Reconcile progress

Add a typed `DiskReconcileProgress` event, emitted only while a snapshot is collected:

- `game_id`, `run_id`, `phase` (`Discovering`, `Indexing`, `Projecting`, `Finalizing`);
- `completed_units`, `total_units` when known;
- `elapsed_ms`, optional `eta_ms`, and `is_indeterminate`.

The snapshot collector reports top-level discovery without a second recursive pre-scan. Once the actual work-unit count is known, it reports determinate progress. ETA is based on elapsed work after a small warm-up, is rounded/coarsened, and is omitted when it is not statistically useful. Emit at percentage changes or a short throttle interval; never once per file.

### Scoped conflict result

Extend `DiskReconcileResult` with the protected scopes and optional blocked action paths required by callers. The backend owns path normalization and ancestor matching. The frontend must not infer safety from display names.

`applyDiskReconcileResult` handles `AppliedWithFolderConflicts` by storing the report, refreshing the safe projection/cache, applying safe path rewrites, and auto-opening the dialog once per report fingerprint. It must not clear the report merely because safe changes applied.

## Dependency Graph

```text
T1 characterization
 ├─ T2 progress protocol + collector instrumentation ── T3 onboarding progress UI
 ├─ T4 grid measurement/virtualizer repair
 └─ T5 protected-scope reconcile writer ── T6 targeted mutation preflight/UI guards
                                     └────── T7 conflict queue lifecycle

T3 + T4 + T6 + T7 ── T8 integration, regression, i18n, and manual desktop check
```

## Tasks

### Task 1 — Characterize the four failure modes

**Description:** Add narrow tests and diagnostic assertions before behavior changes: onboarding reconcile has no progress, maximize/resizing changes the grid container/card metrics, a resolved conflict group disappears from the authoritative report, and a disjoint mod is currently blocked by another conflict.

**Acceptance criteria:**

- [ ] Tests distinguish a real resize layout error from a visual compositor artifact; they record container width, column count, row height, virtual range, and horizontal overflow before and after resize.
- [ ] Rust tests prove that a conflict result today skips unrelated projection writes, establishing the regression case for scoped reconciliation.
- [ ] Frontend tests prove the current global mutation-disabled decision and dialog group replacement behavior.

**Verification:** targeted Rust reconciliation tests and Vitest tests are red only for the intended gaps.

**Dependencies:** None.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/reconcile_tests.rs`
- `src/features/folder-grid/hooks/useFolderGridLayout.test.ts`
- `src/features/folder-grid/modals/FolderConflictManager.test.tsx`
- `src/features/file-watcher/hooks.test.ts`

**Estimated scope:** M.

### Task 2 — Emit bounded, truthful disk-index progress

**Description:** Instrument the existing disk snapshot collection through a small optional progress callback/sink. Preserve the existing no-progress collector wrapper for all non-onboarding callers. Reconcile emits structured phase/progress events with a per-run ID, including a determinate count only after discovery, throttled to avoid event flooding.

**Acceptance criteria:**

- [ ] A first reconcile emits ordered `Discovering → Indexing → Projecting → Finalizing` events and terminal completion/error is unambiguous.
- [ ] No extra recursive pre-scan is introduced solely to calculate a percentage or ETA.
- [ ] ETA is absent until enough samples exist, never negative, and is not emitted more frequently than the configured throttle/percent threshold.
- [ ] Existing watcher/internal reconcile behavior and `disk_reconcile:result` contract remain unchanged except for the additive progress event.

**Verification:** focused Rust tests for phase ordering, throttling, no-ETA warm-up, and collector correctness; generated bindings if the event type is exposed through Specta.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/*`
- `src-tauri/src/services/disk_reconcile/*tests.rs`

**Estimated scope:** M.

### Task 3 — Replace the onboarding spinner with accessible progress and estimate

**Description:** Subscribe only while `WelcomeScreen` is indexing. Aggregate sequential game runs into an overall progress bar, show the current game and phase, and show an honest `Estimating…`/ETA fallback. Always unsubscribe on terminal completion, error, and unmount; do not let stale events update a later onboarding run.

**Acceptance criteria:**

- [ ] The indexing screen has `role="progressbar"`, an announced phase/current game, percentage once known, and a reduced-motion-safe visual treatment.
- [ ] It shows indeterminate discovery first, then determinate progress and ETA only when supplied by the backend.
- [ ] Multiple selected games show completed-game progress plus the active game; a reconcile warning does not leave the screen indefinitely busy.
- [ ] EN/ID/ZH keys cover phases, estimated remaining time, and unavailable ETA.

**Verification:** Vitest event-sequence tests and an onboarding E2E fixture that observes progress from start through completion.

**Dependencies:** Task 2.

**Files likely touched:**

- `src/features/onboarding/WelcomeScreen.tsx`
- `src/features/onboarding/*test.tsx`
- `src/locales/{en,id,zh}/onboarding.json`
- frontend event/type binding files as generated by the project convention

**Estimated scope:** M.

### Task 4 — Make virtualized grid measurement resize-safe

**Description:** Reproduce maximize/fullscreen and pane resizing with the real grid fixture, then fix only the measured fault. The preferred minimal repair is synchronous initial measurement (`useLayoutEffect`), `ResizeObserver` updates, `min-h-0/min-w-0` containment, and a single animation-frame virtualizer measure after a material width/row-height change. If the renderer proves stale transform compositing, use absolute `top` placement for the small number of virtual rows rather than adding a new layout engine or polling loop.

**Acceptance criteria:**

- [ ] Before first visible paint and after every width change, column count, card width, virtual row height, total size, and scroll range derive from the actual scroll container.
- [ ] No horizontal overflow, overlap, stale card image raster, or offset row remains after maximize, restore, sidebar resize, or view-mode switch.
- [ ] Scroll position remains valid (clamped if the new row count is shorter); keyboard navigation uses the updated column count.
- [ ] The fix does not add window polling, CSS scale, or broad `will-change` layers.

**Verification:** hook/component tests for width transitions, E2E resize/maximize fixture assertions, and manual Windows desktop visual verification at normal and high-DPI scaling.

**Dependencies:** Task 1.

**Files likely touched:**

- `src/features/folder-grid/hooks/useFolderGridLayout.ts`
- `src/features/folder-grid/components/FolderGridContent.tsx`
- `src/features/folder-grid/FolderGrid.tsx`
- `src/features/folder-grid/hooks/useFolderGridLayout.test.ts`
- relevant E2E spec

**Estimated scope:** M.

### Task 5 — Reconcile safe paths while quarantining folder conflict scopes

**Description:** Refactor reconcile preflight so it builds the full conflict report, derives protected path scopes (including descendants of conflicting parents), and passes a filtered projection plus protection set to the projection writer. The write transaction applies every safe disk change while preserving valid DB rows and collection state for protected scopes. Return `AppliedWithFolderConflicts` with safe change counts and the current report.

**Acceptance criteria:**

- [ ] A conflict in `Mod A` does not prevent an unrelated add/delete/rename/toggle from being projected in the same game.
- [ ] A parent/object conflict protects all descendants; no protected row is inserted, pruned, identity-rewritten, collection-rewritten, or marked missing during the partial pass.
- [ ] Safe watcher rename hints and runtime projection refresh continue to work; protected hints are deferred until a later clean reconcile.
- [ ] The transaction rolls back fully if safe-scope writer/runtime work fails, while disk remains authoritative for the next run.
- [ ] Removing/renaming the last conflicting candidate yields a clean full reconcile that converges all previously protected rows.

**Verification:** focused Rust fixture matrix for sibling and parent conflicts, 2-/3-candidate groups, safe external mutations, collection references, watcher events, rollback injection, and final clean convergence.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/identity_conflicts.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/{write,objects,mods,prune}.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`
- `src-tauri/src/services/disk_reconcile/*tests.rs`

**Estimated scope:** L, split writer/prune tests from orchestration changes if implementation exceeds one focused session.

### Task 6 — Make conflict preflight and UI guards target-aware

**Description:** Replace the game-wide `ensure_mutation_preflight` decision with path/scope-aware preflight. Each mutation supplies its planned targets; bulk returns safe successes plus blocked paths. The UI derives disabled state from the authoritative protected paths, not from the mere presence of any report. Root-wide plans must calculate targets before writes or explicitly refuse when they cannot do so safely.

**Acceptance criteria:**

- [ ] A safe card can be enabled/disabled, renamed, deleted, edited, and included in a bulk action while another group conflicts.
- [ ] A candidate or descendant of a conflict is visibly marked and rejected by the backend even if a stale UI attempts the action.
- [ ] Bulk responses identify skipped paths without rolling back safe completed paths; UI reports success/skipped counts accurately.
- [ ] Collection apply, auto-organize/category, import/extract, and move actions either use a validated affected-path plan or fail before physical writes with an actionable conflict message.

**Verification:** Rust command tests for direct, parent-child, bulk, collection, import, and auto-organize target scopes; Vitest tests for per-card and bulk disabled/summary behavior.

**Dependencies:** Task 5.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/emit.rs`
- affected `src-tauri/src/commands/**` and mutation services
- `src/features/folder-grid/hooks/useFolderGridViewModel.ts`
- `src/features/folder-grid/components/{FolderGridContent,FolderCard,BulkActionBar}.tsx`
- `src/features/file-watcher/hooks.ts`

**Estimated scope:** L, implement one command family at a time behind the same tested preflight helper.

### Task 7 — Turn conflict resolution into an authoritative completed queue

**Description:** Keep an ephemeral queue session in `FolderConflictManager`: baseline report fingerprint, pending groups, and completed group fingerprints. After rename or Recycle Bin removal, replace pending groups only from the returned reconcile result. Mark a group complete only when it is absent from that latest authoritative report, choose the next unresolved group after the one just acted on, and close only when none remain.

**Acceptance criteria:**

- [ ] The left queue shows `Resolved n / total`, checkmarks for completed groups, candidate count for pending groups, and a clear active/next state.
- [ ] Rename and delete of one candidate immediately refreshes the queue; a still-conflicting three-folder group remains pending, while a resolved group gets a checkmark and advances to the next pending group.
- [ ] An externally changed/new/reappearing report is authoritative: a currently conflicting fingerprint is never shown as completed merely because an older session recorded it.
- [ ] Details requests for the prior group cannot overwrite the newly selected group; busy actions cannot race rename and trash.
- [ ] Dialog status, banner count, per-card conflict styling, cache refresh, and auto-open-once logic remain in sync for `AppliedWithFolderConflicts` and clean `Applied` results.

**Verification:** Vitest tests for rename, trash, three candidates, next selection, external report replacement/reappearance, details request race, and close-on-empty; E2E resolution flow through rename and Recycle Bin action stub.

**Dependencies:** Tasks 5–6.

**Files likely touched:**

- `src/features/folder-grid/modals/FolderConflictManager.tsx`
- `src/features/folder-grid/modals/FolderConflictManager.test.tsx`
- `src/features/file-watcher/hooks.ts`
- `src/features/folder-grid/hooks/useFolderGridViewModel.ts`
- `src/features/folder-grid/components/FolderGridBanners.tsx`
- `src/locales/{en,id,zh}/folder_grid.json`

**Estimated scope:** M.

### Task 8 — Integrate, verify, and document the regression matrix

**Description:** Regenerate bindings if required, run focused then full tests, and validate the combined flows: first onboarding, fullscreen grid, internal/external conflict changes, watcher batch, safe scoped mutations, and final clean reconciliation.

**Acceptance criteria:**

- [ ] There is no global modal/disable state for a safe mod solely because another mod conflicts.
- [ ] `AppliedWithFolderConflicts` performs safe cache/UI refresh, while SourceUnavailable and rename-confirmation behavior retain their existing recovery flows.
- [ ] Command permissions, generated bindings, and `docs/command-permissions.md` remain accurate for any new command/event.
- [ ] No duplicate watcher toast, full-scan storm, or unbounded progress event stream is introduced.

**Verification:**

- [ ] Targeted Rust and Vitest suites for Tasks 2–7.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`.
- [ ] `pnpm test -- --run`, `pnpm lint`, `pnpm i18n:lint`, and `pnpm build`.
- [ ] Relevant WebdriverIO E2E plus manual Windows maximize/high-DPI grid check.
- [ ] `git diff --check` and a focused read-only review.

**Dependencies:** Tasks 2–7.

**Files likely touched:**

- generated bindings/permissions only when the changed contract requires them
- `docs/command-permissions.md`
- targeted test and E2E fixtures

**Estimated scope:** M.

## Checkpoints

### Checkpoint A — Visible progress and stable layout

- [ ] Tasks 1–4 pass targeted checks.
- [ ] Onboarding is informative without lying about ETA.
- [ ] Grid has been verified after maximize/restore and sidebar resize.

### Checkpoint B — Safe partial reconciliation

- [ ] Task 5 proves safe paths reconcile and protected paths are untouched.
- [ ] Task 6 proves backend protection cannot be bypassed by stale UI or bulk commands.

### Checkpoint C — Resolution queue and full regression

- [ ] Task 7 closes/advances exclusively from authoritative reports.
- [ ] Task 8 full verification is complete.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Filtering a conflict parent accidentally prunes or rewrites its children | High | Carry protected ancestor scopes into every writer/prune/collection transition path; include parent-child fixtures and transaction rollback tests. |
| Partial result refresh clears the conflict banner | High | Make `AppliedWithFolderConflicts` a first-class branch in the one result applier; store report before publishing safe cache refresh. |
| Progress emits too often or ETA looks fabricated | Medium | Reuse one collector callback, throttle, phase explicitly, and hide ETA until warm-up. |
| Fullscreen issue is renderer/compositor-specific | Medium | Characterize metrics first; use simple layout fixes first and validate manually on Windows high-DPI before changing virtual row placement. |
| Bulk behavior surprises users | Medium | Return blocked paths and show exact safe/blocked counts; never silently drop targets. |

## Explicit Non-Goals

- No automatic rename/selection of a conflict winner.
- No database migration or compatibility adapter for older desktop versions.
- No polling loop, persistent filesystem journal, background daemon, generic operation framework, or new event bus.
- No full recursive pre-scan solely for a prettier ETA.
