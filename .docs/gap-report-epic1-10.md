# Gap Report: Epic 1-10

Generated: 2026-02-16
Project: EMMM2 Mod Manager
Scope: Compare `.docs` Epic 1-10 requirements against current implementation evidence in `src/` and `src-tauri/`.

---

## Legend

- Implemented: Requirement has clear implementation evidence.
- Partial: Requirement is present but incomplete, mismatched, or not fully wired.
- Missing: No implementation evidence found in this review.

---

## Executive Status Matrix

| Epic                                | Status                        | Notes                                                                                                                             |
| ----------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Epic 1 - Onboarding & Config        | Partial                       | Backend onboarding/validation is present; frontend onboarding slice is not evident in current source scan.                        |
| Epic 2 - Intelligent Scanning       | Partial                       | Core scan + deep matcher pipeline exists; AI stage is not integrated.                                                             |
| Epic 3 - Game/Object Management     | Implemented                   | Schema loading, object commands, and sync pathways are present.                                                                   |
| Epic 4 - Folder Grid & Explorer     | Implemented (with deltas)     | Virtualized grid/list + lazy thumbnails + watcher suppression paths exist; some spec detail mismatches remain.                    |
| Epic 5 - Core Mod Operations        | Implemented                   | Toggle/rename/import/bulk/enable-only-this/conflict checks are implemented with lock and suppression patterns.                    |
| Epic 6 - Preview Panel & INI Editor | Implemented                   | INI list/read/write + backup + preview image pipeline + frontend hooks are present.                                               |
| Epic 7 - Privacy Mode               | Implemented                   | Safe mode + PIN hash/verify + filtering controls are present.                                                                     |
| Epic 8 - Collections                | Implemented (with constraint) | Apply/undo/snapshot exists; undo snapshot is in-memory (single-session scope).                                                    |
| Epic 9 - Duplicate Scanner          | Implemented                   | blake3 + rayon duplicate scanner and resolve flow are implemented.                                                                |
| Epic 10 - QoL Automation            | Partial                       | Backend commands exist (launch/random/favorite), but frontend invoke call-sites for launch/random were not found in current scan. |

---

## Epic-by-Epic Gap Details

## Epic 1 - Onboarding & Config

Status: **Partial**

### Evidence

- Auto-detect/manual add game commands: `src-tauri/src/commands/game_cmds.rs`
- Instance validation rules (`Mods`, `d3dx.ini`, `d3d11.dll`, launcher exe): `src-tauri/src/services/validator.rs`
- Single-instance plugin registration: `src-tauri/src/lib.rs`
- Config service + PIN/safe settings persistence infrastructure: `src-tauri/src/services/config/mod.rs`

### Gaps

- Frontend onboarding flow/components are not evident from current scan (no concrete onboarding feature files discovered under `src/features/onboarding`).

### Risk

- Backend capabilities may be underutilized if first-run UX wiring is incomplete.

---

## Epic 2 - Intelligent Mod Scanning & Organization

Status: **Partial**

### Evidence

- Scan command orchestration and pipelines: `src-tauri/src/commands/scan_cmds.rs`
- Folder walking and archive detection: `src-tauri/src/services/scanner/walker.rs`
- Deep matcher engine: `src-tauri/src/services/scanner/deep_matcher.rs`
- Sync to DB/objects: `src-tauri/src/services/scanner/sync.rs`
- Archive extract + smart flatten + backup: `src-tauri/src/services/file_ops/archive/extract.rs`

### Gaps

- AI stage (L4 in requirement narrative) is not evidenced as integrated into production flow.

### Risk

- Match quality for ambiguous cases may rely only on deterministic heuristics/fuzzy path.

---

## Epic 3 - Game & Object Management

Status: **Implemented**

### Evidence

- Schema loading + fallback mechanics: `src-tauri/src/services/schema_loader.rs`
- Object command surface: `src-tauri/src/commands/object_cmds.rs`
- Search worker presence for frontend search path: `src/workers/searchWorker.ts`
- Sync persistence patterns: `src-tauri/src/services/scanner/sync.rs`

### Gaps

- None critical found in this pass for core Epic 3 scope.

---

## Epic 4 - Folder Grid & Advanced Explorer

Status: **Implemented (with deltas)**

### Evidence

- Virtualized explorer/grid rendering: `src/features/explorer/FolderGrid.tsx`
- Virtualizer setup (`useVirtualizer`): `src/features/explorer/hooks/useFolderGrid.ts`
- Lazy per-card thumbnail hook (`get_mod_thumbnail`): `src/hooks/useThumbnail.ts`
- Card usage of lazy thumbnail stream: `src/features/explorer/FolderCard.tsx`
- Backend lazy thumbnail command: `src-tauri/src/commands/folder_cmds.rs`
- Thumbnail cache pipeline: `src-tauri/src/services/images/thumbnail_cache.rs`
- Watcher suppression support: `src-tauri/src/services/watcher.rs`

### Gaps / Deltas

- Requirement text and implementation differ in some cache details (doc wording around L2 format/strategy vs current WebP cache implementation).
- Some target UX metrics/checklist expectations are not explicitly proven in this static evidence pass.

### Risk

- Documentation drift can create false-negative QA outcomes unless Epic spec or implementation contract is aligned.

---

## Epic 5 - Core Mod Management

Status: **Implemented**

### Evidence

- Toggle/rename/delete/import/bulk operations: `src-tauri/src/commands/mod_cmds.rs`
- Enable-only-this + duplicate/shader conflict checks: `src-tauri/src/commands/epic5_cmds.rs`
- Archive ingestion from file paths: `src-tauri/src/commands/mod_cmds.rs`
- Archive extraction helpers: `src-tauri/src/services/file_ops/archive/extract.rs`
- Frontend operation hooks/mutations: `src/hooks/useFolders.ts`

### Gaps

- No critical missing core operation identified in this pass.

---

## Epic 6 - Preview Panel & INI Editor

Status: **Implemented**

### Evidence

- Preview panel UI and controls: `src/features/details/PreviewPanel.tsx`
- Preview state/data hooks: `src/features/details/hooks/usePreviewPanelState.ts`, `src/features/details/hooks/usePreviewData.ts`
- INI parser and safe read model: `src-tauri/src/services/file_ops/ini_document.rs`
- INI write path with backup + atomic temp replace: `src-tauri/src/services/file_ops/ini_write.rs`
- Preview command bridge (list/read/write ini, image ops): `src-tauri/src/commands/preview_cmds.rs`
- Preview image processing and naming: `src-tauri/src/services/file_ops/preview_image.rs`

### Gaps / Deltas

- Spec naming and implementation naming for backup extensions may differ in wording (`.backup` vs `.bak` style references); functionality exists.

---

## Epic 7 - Privacy Mode

Status: **Implemented**

### Evidence

- PIN hash verification and lockout state: `src-tauri/src/services/config/pin_guard.rs`
- Safe mode settings + PIN hash persistence: `src-tauri/src/services/config/mod.rs`
- Settings command bridge for PIN and safe mode: `src-tauri/src/commands/settings_cmds.rs`
- Safe filtering in folder listing path: `src-tauri/src/commands/folder_cmds.rs`
- `is_safe` metadata lifecycle: `src-tauri/src/services/file_ops/info_json.rs`

### Gaps

- No critical missing core privacy primitives found in this pass.

---

## Epic 8 - Collections

Status: **Implemented (with constraint)**

### Evidence

- Collections service and storage: `src-tauri/src/services/collections/storage.rs`
- Apply + undo snapshot logic: `src-tauri/src/services/collections/apply.rs`
- Undo state model: `src-tauri/src/services/collections/types.rs`
- Command bridge: `src-tauri/src/commands/collection_cmds.rs`
- Frontend hooks/page: `src/features/collections/hooks/useCollections.ts`, `src/features/collections/CollectionsPage.tsx`

### Gaps / Constraints

- Undo snapshot is in-memory and session-scoped; no persistent multi-level undo history observed.

### Risk

- Undo may not survive app restart/crash scenarios.

---

## Epic 9 - Duplicate Scanner

Status: **Implemented**

### Evidence

- Duplicate scan orchestration and tasking: `src-tauri/src/services/scanner/dedup_scanner.rs`
- Signal extraction + blake3 hashing: `src-tauri/src/services/scanner/dedup_scanner_signals.rs`
- Scan command and event stream: `src-tauri/src/commands/dup_scan_cmds.rs`
- Resolve command + resolver pipeline: `src-tauri/src/commands/dup_resolve_cmds.rs`, `src-tauri/src/services/scanner/dedup_resolver.rs`
- Frontend service bridge: `src/services/dedupService.ts`

### Gaps

- No critical missing dedup core path found in this pass.

---

## Epic 10 - QoL Automation

Status: **Partial**

### Evidence

- Launch command (loader + game): `src-tauri/src/commands/game_cmds.rs`
- Random pick command: `src-tauri/src/commands/mod_cmds.rs`
- Favorite toggle command: `src-tauri/src/commands/mod_cmds.rs`
- Command registration: `src-tauri/src/lib.rs`

### Gaps

- Current scan did not find frontend invoke call-sites for `launch_game` or `pick_random_mod` in `src/`.
- Pinning/favorite paths have mixed semantics (`pin_mod`, `toggle_favorite`, `is_pinned`, `is_favorite`) and should be standardized.

### Risk

- Backend capabilities can remain dormant without frontend wiring.
- Semantic drift in pin/favorite fields can cause inconsistent UX and data expectations.

---

## Prioritized Remediation Backlog

## P0 (Highest)

1. Wire Epic 10 frontend actions to existing backend commands (`launch_game`, `pick_random_mod`) and verify end-to-end flows.
2. Standardize pin/favorite model (`is_pinned` vs `is_favorite`) across DB, commands, and frontend state.

## P1

1. Resolve Epic 2 AI-stage expectations: either implement L4 integration or explicitly revise spec scope.
2. Align Epic 4 cache implementation contract with the written spec to prevent QA mismatch.
3. Decide whether Epic 8 undo must persist across sessions; if yes, add persistent snapshot storage.

## P2

1. Reconcile documentation wording deltas in Epic 6 backup naming and other minor contract details.
2. Add explicit AC-to-test traceability matrix in `.docs` for Epics 1-10.

---

## Method Notes

- This report is evidence-based from code scanning and file inspection, not runtime validation.
- For unresolved items marked Partial/Missing, absence means "not found in this review pass" and should be confirmed with targeted verification tests.
