# E2E Test Scope — Phased Plan (WDIO + tauri-driver)

> Scenario source of truth: `tc-NN-*.md` files in this folder. This document **only organizes E2E order and scope**; it does not rewrite test cases.
> Harness: `wdio.conf.ts` → `tauri-driver` + `msedgedriver`, `maxInstances: 1` (serial); requires a debug build at `src-tauri/target/debug/emmm.exe`. Run: `pnpm test:e2e`.
> Principle: **Disk = source of truth, DB = index**. Every phase that touches disk MUST use a sandbox folder (a small copy), not the original library.

---

## Implementation status

**Phases 0–9 are implemented** (IPC-driven + two-sided disk/DB assertions). Gate: `tsc` + `eslint` are green. They have not been executed at runtime (requires a debug build + Windows driver).

| Spec file | Phase | Epic (tc-NN) |
| -------------------------------- | ---- | -------------------------------- |
| `app.e2e.ts`                     | 0    | boot + severe-error gate         |
| `app-bootstrap.e2e.ts`           | 0    | 01                               |
| `smoke-data-safety.e2e.ts`       | 0    | 00 (toggle/trash/reconcile)      |
| `onboarding.e2e.ts`              | 1    | 03                               |
| `game-management.e2e.ts`         | 1    | 02                               |
| `phase1-gates.e2e.ts`            | 1    | 04, 02 (multi-game/switch)       |
| `phase2-navigation.e2e.ts`       | 2    | 07, 08, 09, 11                   |
| `phase3-mod-ops.e2e.ts`          | 3    | 10, 13, 14, 20, 21, 22, 40       |
| `phase4-preview.e2e.ts`          | 4    | 16, 17, 18, 19, 41, 43           |
| `phase5-import.e2e.ts`           | 5    | 23, 37, 38, 39                   |
| `phase6-scan-sync.e2e.ts`        | 6    | 25, 26, 27, 28, 32               |
| `phase7-collections.e2e.ts`      | 7    | 29, 30, 31                       |
| `phase8-runtime.e2e.ts`          | 8    | 33, 34*, 35, 36, 42*             |
| `phase9-security-browser.e2e.ts` | 9    | PIN(read-only)*, themes, browser |

Shared support: `tests/e2e/support/{fixtures,ipc,app,data}.ts`. Archive fixture: `tests/e2e/fixtures/sample-mod.zip`. `*` = partially [manual-smoke], see below.

### Runtime notes (MUST be read before `pnpm test:e2e`)

1. **The DB is not isolated between runs.** Specs write to the real `app_data` DB (games/objects/collections/trash accumulate). Run against a **debug** build with a disposable `app_data`, or call `reset_database` in the global `before` hook for a clean state. Mod fixture folders in temp are already auto-cleaned.
2. **IPC channels** (`invokeWithChannel` for scanner/dedup/extract) rely on `window.__TAURI__.core.Channel` — available because `withGlobalTauri: true` is set in `tauri.conf.json`.
3. **Alphabetical serial order** (`maxInstances: 1`): `app` → `app-bootstrap` → `game-management` → `onboarding` → `phase1..9` → `smoke`. Each spec seeds its own game and uses unique object names, minimizing state leakage.
4. **[manual-smoke]** (not automated; must be verified manually): `launch_game` (spawns the real loader), real updater (`tc-34` only checks that the command settles), in-game hotkey (`tc-42` only checks registration), extraction of **password-protected/7z** archives (`tc-37` only covers ordinary zip), and **set/verify/reset PIN** (`tc-45` — intentionally read-only: a stale PIN can lock the app at boot).

---

## Harness rules (apply to all phases)

1. **Per-spec isolation.** Each spec creates its own mock game folder in `os.tmpdir()` (the pattern already exists in `game-management.e2e.ts` / `onboarding.e2e.ts`), seeds it via `invoke('add_game_manual', …)`, and removes the folder in `after()`. Do not mutate the user's DB/library.
2. **IPC bypass for actions that cannot be clicked.** For the native file picker, launching the real game, the in-game overlay, and archive password prompts, call the Rust command directly through `window.__TAURI__.core.invoke`; do not simulate OS dialogs.
3. **Selector contract.** Prefer `[data-testid=…]` / `id`. If a target element has no testid yet, add one to the component (as part of that phase's work); do not rely on text that can change with i18n.
4. **Assert both sides.** For disk operations, verify **disk** (the `DISABLED ` prefix, trash location) **and** UI/DB after reconciliation. See `docs/flow.md`.
5. **Serial + idempotent.** With `maxInstances: 1`, specs within a phase must not leak state to one another. Run the kill-app test (tc-00 #6) last in its phase.
6. **Be explicit about limits.** Actions that cannot be fully automated (real updater, in-game hotkey, launching the real game) are marked **[manual-smoke]** — do not claim they are covered.

---

## Phase 0 — Harness foundation & data-safety smoke

**Goal:** Fixture infrastructure and safety nets before feature coverage. **Prerequisite:** a debug build is available.
**Deliverable:** Shared fixture helpers + an automated release smoke checklist where possible.

| Item | Ref | E2E notes |
| --------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| Extract shared fixture helpers (mock game factory, IPC seeding, cleanup) | —            | Refactor from the two existing specs into `tests/e2e/support/`. DRY; used by every phase. |
| Production smoke path (data safety) | tc-00 | Combine six scenarios into one "smoke" spec covering toggle/trash/import/apply at a minimal end-to-end level. Expand the rest in their specific phases. |
| Boot + no-severe-error gate | tc-01, `app` | Make `app.e2e.ts` fail hard on any React `SEVERE` error. |

**Exit:** Fixture helpers are reused; `pnpm test:e2e` passes serially.

---

## Phase 1 — Entry gates (Onboarding · Game · Settings)

**Goal:** All paths leading into the dashboard. **Prerequisite:** Phase 0.

| Epic | Ref | Flow focus |
| ----------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Onboarding / FTUE | tc-03 ✅expand | Auto-detect failure → manual add → dashboard transition; validate core files (d3dx.ini/d3d11.dll/loader). |
| Game Management | tc-02 ✅expand | Add/edit/remove game, switch active game, Quick Play launch (**[manual-smoke]** for the real game process — E2E only needs to assert the command was invoked). |
| Settings | tc-04 | Change theme, language (runtime i18n), and path settings; persist to the store and reload. |

**Exit:** From a fresh state, the dashboard is reachable through all three paths; setting changes persist across restarts.

---

## Phase 2 — Navigation & read surfaces (Workspace · ObjectList · Folder listing)

**Goal:** Read-only projections; low risk and broad navigation coverage. **Prerequisite:** Phase 1 (requires a seeded game and mods).

| Epic                             | Ref   |
| -------------------------------- | ----- |
| Workspace layout & navigation    | tc-05 |
| ObjectList navigation & resizing | tc-06 |
| Object List                      | tc-07 |
| Smart filters & sorting          | tc-08 |
| Object schema & master DB        | tc-09 |
| Folder listing & classification | tc-11 |
| Folder Grid UI                   | tc-12 |

**Seed:** A mock game with objects and nested mods (Object → Skin → Variants), following the hierarchy in `docs/flow.md`.
**Exit:** Object-to-folder navigation, filter/sort, and panel resizing — all without disk mutations.

---

## Phase 3 — Core mod operations ⚠️ DATA SAFETY (highest risk)

**Goal:** All destructive disk mutations. **Prerequisite:** Phase 2. Disk + DB assertions are mandatory.
**Maps to tc-00:** #1 toggle, #2 trash, #5 file-locked, #6 kill-mid-bulk.

| Epic | Ref | Critical verification |
| ------------------------------------ | ----- | -------------------------------------------------------------------------------- |
| Core mod ops (toggle/rename/delete) | tc-13 | Shared root-cause path. |
| Mod Toggle | tc-20 | Correct `DISABLED ` prefix, no missing files, UI in sync after reconciliation. |
| Mod Rename | tc-21 | Rename while a file is locked → retry dialog; folder remains intact. |
| Trash Safety | tc-22 | Move to `app_data/trash/`, restore intact, **no hard delete**. |
| Object CRUD | tc-10 | Create/rename/delete objects consistently across disk and DB. |
| Metadata actions (pin/favorite/move) | tc-40 | Moving between objects does not remove the mod. |
| Bulk operations & selection | tc-14 | Force-kill the app during a bulk operation → restart + consistent reconciliation (run last). |

**Exit:** All operations are reversible/safe; post-crash reconciliation is consistent.

---

## Phase 4 — Preview & editor

**Goal:** Right panel and content editor. **Prerequisite:** Phase 2 (read coverage) — some tests write INI files.

| Epic | Ref | Notes |
| ---------------------------- | ----- | ----------------------------------- |
| Preview panel layout & state | tc-16 |                                     |
| Metadata editor | tc-17 | Write metadata → verify persistence. |
| INI viewer & editor | tc-18 | Edit → save INI file (assert disk state). |
| Image gallery                | tc-19 |                                     |
| Thumbnail cache | tc-41 | Cache hit/miss and invalidation. |
| Dynamic KeyViewer overlay | tc-43 | Parse keys from INI → render. |

**Exit:** Metadata/INI edits persist to disk; gallery and thumbnails render.

---

## Phase 5 — Import & organization ⚠️ DATA SAFETY

**Goal:** New mod intake pipeline and collision handling. **Prerequisite:** Phase 3 (trash/rename are mature). **Maps to tc-00:** #3 import, #6.

| Epic | Ref | Critical verification |
| --------------------------------- | ----- | --------------------------------------------------------------------- |
| Mod import pipeline | tc-23 | Import .zip/.7z; verify extraction results. |
| Archive extraction | tc-37 | Includes password-protected archives (**[IPC-bypass]** for password input). |
| Folder collision resolution | tc-39 | A collision dialog appears on name conflicts and respects the user's choice. |
| Auto-organizer | tc-38 | Automatically place mods in the matching object. |
| Explorer interactions (drag/drop) | tc-15 | Drag-and-drop import (**[IPC-bypass]** if OS-level drop cannot be simulated). |

**Exit:** Import → extract → resolve collisions → mod appears in the grid, with no silent file overwrites.

---

## Phase 6 — Scan / sync / matching engine (backend-heavy)

**Goal:** Command-level E2E (invoke → assert DB/disk), tolerant of long-running operations. **Prerequisite:** Phase 2.

| Epic                             | Ref   |
| -------------------------------- | ----- |
| Scan engine                      | tc-25 |
| Deep matcher & auto-categorization | tc-26 |
| Sync database (disk reconcile)   | tc-27 |
| File watcher (external drift) | tc-28 |
| Smart duplicate scanner          | tc-32 |

**Note:** For the file watcher, change disk state outside the app (direct filesystem access) and assert that UI/DB update. For large scans, increase the per-spec Mocha timeout.
**Exit:** Scans populate the DB correctly; external disk drift is detected and reconciled.

---

## Phase 7 — Collections & conflicts ⚠️ DATA SAFETY (transactional)

**Goal:** Transactional apply and privacy. **Prerequisite:** Phases 3 + 6. **Maps to tc-00:** #4 partial apply, #7 ignore duplicate.

| Epic | Ref | Critical verification |
| ------------------------------- | ----- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Virtual collections | tc-31 | Apply with some mods missing → transactional behavior, partial failure displayed, **no half-applied state**. Recovery resumes after a crash. |
| Conflict detection & resolution | tc-29 | Force enable + "don't remind me again" → warning stays suppressed; entry appears in IgnoreManagementModal and can be revoked. |
| Privacy & Safe Mode | tc-30 | Switch safe mode: preview/runtime parity; sensitive content is hidden. |

**Exit:** Collection apply is always all-or-nothing with a report; safe mode stays consistent between disk and runtime.

---

## Phase 8 — Peripheral runtime features (Randomizer · Dashboard · Updater · Toast)

**Goal:** Cross-cutting features, some of which require manual checks. **Prerequisite:** Phases 3/7.

| Epic | Ref | Notes |
| -------------------------------------- | ----- | ----------------------------------------------------------------------- |
| Smart randomizer + integrated launcher | tc-35 | Randomization selects a valid mod; launch is **[manual-smoke]**. |
| In-game hotkeys & live controls | tc-42 | **[manual-smoke]** in-game overlay; E2E only needs to assert hotkey registration. |
| Dashboard & analytics | tc-33 | Aggregate counts are correct for the seeded data. |
| App updater & asset sync | tc-34 | **[manual-smoke]** real update; E2E mocks the check-update command. |
| Toast & error handling pipeline | tc-36 | Trigger a controlled error → toast appears; no silent failure. |

**Exit:** Peripheral paths are covered; manual items are clearly documented.

---

## Coverage matrix (all areas mapped)

| Phase | Epic (tc-NN) |
| ---- | -------------------------- |
| 0    | 00, 01, (app smoke)        |
| 1    | 02, 03, 04                 |
| 2    | 05, 06, 07, 08, 09, 11, 12 |
| 3    | 10, 13, 14, 20, 21, 22, 40 |
| 4    | 16, 17, 18, 19, 41, 43     |
| 5    | 15, 23, 37, 38, 39         |
| 6    | 25, 26, 27, 28, 32         |
| 7    | 29, 30, 31                 |
| 8    | 33, 34, 35, 36, 42         |

_(tc-24 is not in the repo — this is not a gap.)_

**Recommended execution order:** 0 → 1 → 2 → 3 → 4/6 (work can proceed in parallel, but runs are serial) → 5 → 7 → 8.
Phases ⚠️ (3, 5, 7) are release gates: do not dogfood against the original library until all three pass in the sandbox.

---

## Scenario coverage audit (2026-07-21)

Of **277 documented scenarios** (`TC-NN-XX` across all tc-docs), **48 `it()` cases are automated** (~17%). The number is low because most documented scenarios are pure UI interactions and minor variants. Qualitatively, **41/43 areas have at least one happy-path E2E**; edge/failure/UI depth is still lacking.

### Tier A — Automatable via IPC (high value)

Some are **already covered** in `phase3b-failure-paths.e2e.ts` (reserved-character rename rejection, duplicate-object rejection, bulk partial failure, empty_trash). Remaining worthwhile additions:

- **tc-02 (22 docs, 2 automated):** edit/remove game, auto-detect, invalid path/core-file validation.
- **tc-04 (23 docs, 2 automated):** persistence for each tab (maintenance, keyviewer, hotkey config, AI), reset.
- **tc-01 (18 docs, 1 automated):** recovery-task resume, config-state branching, splash close.
- **tc-10/07/08/11/18/25:** CRUD variants, filter/sort, folder classification, malformed INI, large scans.
- **Remaining tc-13:** locked-folder rollback (#TC-13-003, requires a lock handle), rapid-toggle idempotency (#TC-13-004).

### Tier B — Requires `data-testid` + click/drag automation (deferred; modify component source)

Nearly all **zero-automation areas**: `tc-05` (12), `tc-06` (11), `tc-12` (15), `tc-15` (12), `tc-16` (11) — divider resize + persistence, context menu, breadcrumb, mobile <768px, drag-rectangle selection, panel collapse. Add testids to real components first (there are only ~11 testids in the production DOM now), then add WDIO click/drag specs.

### Tier C — Better suited to unit tests (not E2E)

`tc-26` (18 matcher scenarios: alias/token/hash overlap, tiebreak, AI, GameBanana) and some `tc-25` scoring — algorithm accuracy is best covered by Rust unit tests. E2E only needs to prove the pipeline runs (already covered: TC-25-01).

### Tier D — Manual smoke (already marked)

`launch_game`, real updater, in-game hotkey, password-protected/7z archives, set/verify/reset PIN.

**Recommended next steps:** complete Tier A (add IPC specs only; low risk) → decide on Tier B (requires component-source changes) → move Tier C to unit tests.

---

## Gap-closure progress (2026-07-21, continued)

### Tier A — COMPLETE (new IPC specs; no app source changes)

- `phase3b-failure-paths.e2e.ts` — rename reserved-char/empty reject, duplicate object reject, bulk partial-failure, empty_trash (tc-13/10/14/22 edges).
- `phase0b-bootstrap.e2e.ts` — config-status `HasConfig`, startup recovery check (tc-01).
- `phase1b-game-edit.e2e.ts` — edit/remove game via save_settings, auto-detect, auto-close-launcher persist (tc-02).
- `phase1c-settings-depth.e2e.ts` — maintenance, clear thumbnails, hotkey config, keyviewer persist (tc-04).
- `phase2b-navigation-edges.e2e.ts` — master-DB search, enabled/disabled folder classification (tc-08/11).
- `phase4b-editor-edges.e2e.ts` — tags add/remove round-trip, multi-INI listing (tc-17/18).

### Tier B — Click-driven UI COMPLETE (landmarks + selection + view toggle)

Additive `data-testid` attributes (safe — 169 + 56 unit tests still pass) on: TopBar nav (`nav-<id>`), `ResizableWorkspace` (`workspace-desktop/left/main/right`, `resize-handle-left/right`), `ObjectList` (`object-list-panel`), `FolderGrid` (`folder-grid`), and `FolderGridToolbar` (`view-grid`/`view-list`). Object rows use the existing `data-object-id`.

- `phaseUI-workspace.e2e.ts` — nav App Menu → mods, assert 3-pane landmark mount + switch dashboard↔mods.
- `phaseUI-folder-grid.e2e.ts` — click an object row → FolderGrid mounts (single-click → focusObject → selectedObjectFolderPath), toggle grid/list.

**Remaining Tier B (intentionally left manual/flaky):** pixel-drag resize + persistence (tc-05-01/tc-06-01/tc-16-006), drag-rectangle marquee selection (tc-15-005), mobile <768px viewport (tc-06-07..09). Pixel/drag gestures are inherently flaky in E2E → manual/visual QA. Landmark testids (`resize-handle-*`) are already in place if this is revisited later.

### Tier C — tc-26 matcher: NOT a gap. All 82 Rust unit tests already pass (`deep_matcher/tests/**` — alias/token/hash overlap, tiebreak, evidence gate, negative penalty, gamebanana, AI rerank, malformed INI). No new E2E spec is needed.

### Tier D — Manual smoke remains: launch_game, real updater, in-game hotkey, password-protected/7z archives, set/verify/reset PIN.
