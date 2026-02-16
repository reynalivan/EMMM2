# Epic 6 Preview Panel and Detail View (TDD Work Plan)

## TL;DR

> **Quick Summary**: Deliver Epic 6 by replacing the current PreviewPanel stub with real data flows, adding a lossless Rust INI editing pipeline, and enforcing safe file operations with lock + backup + atomic write.
>
> **Deliverables**:
>
> - Real data-driven Preview Panel (metadata, gallery, INI editor, dirty-state guard)
> - Rust INI service (lossless parse/edit, encoding-safe, BOM-aware, backup + atomic save)
> - Tauri command bridge for image/INI/detail operations
> - TC-mapped test suite (frontend + backend) and agent-executed QA evidence
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: 1 -> 2 -> 3 -> 5 -> 6 -> 7 -> 9 -> 10

---

## Context

### Original Request

- Start Epic 6 planning workflow (`.agent/workflows/start-epic.md`) with TDD.
- Align to `.docs/epic6-previewpanel.md`, `.docs/.testcase/TC-Epic-06-PreviewPanel.md`, and `.docs/trd.md`.
- Maximize discovery and produce an executable work plan.

### Interview Summary

**Key Discussions**:

- User confirmed TDD mode (RED-GREEN-REFACTOR).
- User selected compatibility-first conflict policy for BOM/backup/image format.

**Research Findings**:

- `src/features/details/PreviewPanel.tsx` is a hardcoded stub.
- `src/components/layout/MainLayout.tsx` already mounts the preview pane, so wiring exists.
- `src/hooks/useFolders.ts` already includes `useUpdateModInfo` and `usePasteThumbnail` hooks.
- `src-tauri/src/commands/mod_cmds.rs` already exposes `read_mod_info`, `update_mod_info`, `paste_thumbnail`.
- `src-tauri/src/services/file_ops/info_json.rs` and `src-tauri/src/services/operation_lock.rs` provide reusable foundations.
- No dedicated INI editor service/module currently exists.

### Metis Review

**Identified Gaps (addressed in this plan)**:

- Potential spec ambiguity around BOM/backup/image output format.
- Risk of data corruption without strict lock + atomic-write + backup sequence.
- Scope creep risk into rich INI UX beyond Epic 6 goals.

---

## Work Objectives

### Core Objective

Deliver a production-safe Epic 6 Preview experience with metadata editing, image gallery management, and 3DMigoto INI editing while preserving data integrity and performance constraints from TRD.

### Concrete Deliverables

- `src/features/details/` slice expanded beyond stub to real data modules.
- New Rust service(s) for INI discovery/read/edit/save with lossless behavior.
- New/updated Tauri commands for details panel workflows.
- Automated tests mapped to TC-6._ / NC-6._ / EC-6._ / TM-6._ / DI-6.\*.
- QA evidence artifacts under `.sisyphus/evidence/`.

### Definition of Done

- [ ] Epic 6 test cases are traceably covered in automated tests and QA scenarios.
- [ ] No data-loss path in INI/info/image operations (backup + atomic write + lock).
- [ ] Preview panel transitions and key interactions remain within TRD latency targets.

### Must Have

- TDD-first sequencing for all implementation tasks.
- Agent-executable verification only (no human-only validation steps).
- Compatibility-first policy:
  - BOM: strip in-memory for parsing, preserve original BOM state on write.
  - Backup: `.ini.bak`.
  - Pasted image target: `preview_custom.png` for v1.
  - Encoding default: detect UTF-8 (BOM/no BOM) first, then Shift-JIS/GBK fallback; if decode confidence is unsafe, open read-only/raw fallback and block save.

### Must NOT Have (Guardrails)

- No broad parser rewrite that normalizes or reorders unrelated INI lines.
- No introduction of new dependencies without explicit approval.
- No cross-layer leakage (frontend must not touch `src-tauri` internals directly).
- No scope expansion into full syntax-highlighted code-editor platform work.

---

## Verification Strategy (MANDATORY)

> **UNIVERSAL RULE: ZERO HUMAN INTERVENTION**
>
> All tasks are verifiable via command/tool execution by the agent.

### Test Decision

- **Infrastructure exists**: YES
- **Automated tests**: TDD
- **Framework**:
  - Frontend: Vitest + RTL (`pnpm test`)
  - Backend: cargo test (`cd src-tauri && cargo test`)

### If TDD Enabled

Each task follows RED -> GREEN -> REFACTOR with task-level commands.

### Agent-Executed QA Scenarios (MANDATORY)

All tasks include concrete happy-path + failure-path scenarios with evidence capture.

---

## Execution Strategy

### Parallel Execution Waves

Wave 1 (Foundation):

- Task 1 (spec freeze + contracts)
- Task 2 (INI read model)
- Task 4 (image policy/discovery backend alignment)

Wave 2 (Safe write + command bridge):

- Task 3 (INI write/backup/atomic)
- Task 5 (new/updated commands)

Wave 3 (Frontend integration):

- Task 6 (details hooks/query layer)
- Task 7 (metadata panel + debounced autosave)
- Task 8 (gallery + paste + lazy loading)

Wave 4 (Editor, guard, hardening):

- Task 9 (INI editor + dirty-state guard + validation)
- Task 10 (TC-mapped regression, perf, evidence pack)

### Dependency Matrix

| Task | Depends On | Blocks      | Can Parallelize With |
| ---- | ---------- | ----------- | -------------------- |
| 1    | None       | 3,5,6,7,8,9 | 2,4                  |
| 2    | 1          | 3,5,9       | 4                    |
| 3    | 2          | 5,9,10      | None                 |
| 4    | 1          | 5,8,10      | 2                    |
| 5    | 2,3,4      | 6,7,8,9     | None                 |
| 6    | 5          | 7,8,9       | None                 |
| 7    | 6          | 10          | 8                    |
| 8    | 6          | 10          | 7                    |
| 9    | 3,5,6      | 10          | None                 |
| 10   | 7,8,9      | None        | None                 |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Agents                                                                                                              |
| ---- | ----- | ------------------------------------------------------------------------------------------------------------------------------- |
| 1    | 1,2,4 | task(category="unspecified-high", load_skills=["tdd","ini-parser","atomic-fs"], run_in_background=false)                        |
| 2    | 3,5   | task(category="backend-development", load_skills=["tauri-command","atomic-fs","tdd"], run_in_background=false)                  |
| 3    | 6,7,8 | task(category="visual-engineering", load_skills=["frontend-ui-ux","tdd"], run_in_background=false)                              |
| 4    | 9,10  | task(category="unspecified-high", load_skills=["writing-unit-tests","verification-before-completion"], run_in_background=false) |

---

## TODOs

- [x] 1. Freeze Epic 6 contract and testcase mapping

  **What to do**:
  - Build a TC mapping table from `TC-6.1-01` through `DI-6.03`.
  - Lock conflict decisions (BOM/backup/image format) in plan notes used by implementer.
  - Define explicit IN/OUT scope boundaries for v1.

  **Must NOT do**:
  - Do not defer conflict decisions to implementation time.

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: contract and testcase normalization is documentation-first.
  - **Skills**: `writing-plans`, `tdd`
    - `writing-plans`: structured, enforceable task breakdown.
    - `tdd`: ensures every contract item maps to verification.
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: not needed yet (no UI construction in this task).

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: 3,5,6,7,8,9
  - **Blocked By**: None

  **References**:
  - `.docs/epic6-previewpanel.md` - feature expectations and acceptance definitions.
  - `.docs/.testcase/TC-Epic-06-PreviewPanel.md` - required TC IDs to map and verify.
  - `.docs/trd.md` - architecture and operation lock/INI constraints.

  **Acceptance Criteria**:
  - [x] Contract table exists with each TC/NC/EC/TM/DI mapped to a future task.
  - [x] Conflict policies are explicitly documented and unambiguous.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Contract completeness check
    Tool: Bash
    Preconditions: Repository available
    Steps:
      1. Open contract mapping markdown.
      2. Verify every testcase ID from TC file appears exactly once.
      3. Assert conflict policy section contains BOM/backup/image decisions.
    Expected Result: No unmapped testcase IDs.
    Evidence: .sisyphus/evidence/task-1-contract-check.txt

  Scenario: Contract conflict omission detection
    Tool: Bash
    Preconditions: Contract mapping file present
    Steps:
      1. Search contract document for keywords: BOM, .ini.bak, preview_custom.png.
      2. Assert all three terms exist in conflict-policy section.
    Expected Result: Missing policy terms trigger failure before execution starts.
    Evidence: .sisyphus/evidence/task-1-contract-negative.txt
  ```

- [x] 2. Build lossless INI read/discovery service (Rust)

  **What to do**:
  - Add service for listing valid `.ini` files in mod root (excluding known noise like `desktop.ini`).
  - Add read parser model that captures line index, section context, keybindings (`[Key...] key/back`), and `$variables`.
  - Detect BOM/newline/encoding state metadata for safe round-trip.

  **Must NOT do**:
  - Do not normalize/reformat full file content.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `ini-parser`, `tdd`, `backend-development`
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: not backend parsing.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: 3,5,9
  - **Blocked By**: 1

  **References**:
  - `src-tauri/src/services/file_ops/info_json.rs` - pattern for file lifecycle and tests.
  - `src-tauri/src/services/scanner/conflict.rs` - INI scanning style and parser test patterns.
  - `.docs/trd.md` - custom 3DMigoto parser requirements and operation constraints.

  **Acceptance Criteria**:
  - [x] Rust tests prove parser can detect keybindings + variables + line index metadata.
  - [x] Parser reads BOM-containing files and strips BOM for in-memory parsing only.
  - [ ] Command-level benchmark target for 500-line INI parse <= TM-6.01 threshold.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Parse valid INI with key + variable
    Tool: Bash
    Preconditions: New parser tests implemented
    Steps:
      1. Run: cd src-tauri && cargo test ini_read_model_detects_keys_and_variables -- --nocapture
      2. Assert test output includes parsed key section and variable count > 0.
    Expected Result: Test passes and reports expected extraction.
    Evidence: .sisyphus/evidence/task-2-parse-happy.txt

  Scenario: Parse malformed INI fallback model
    Tool: Bash
    Preconditions: Negative testcase exists
    Steps:
      1. Run: cd src-tauri && cargo test ini_read_model_handles_malformed -- --nocapture
      2. Assert no panic and fallback/raw-mode indicator returned.
    Expected Result: Safe failure path with recoverable model.
    Evidence: .sisyphus/evidence/task-2-parse-negative.txt
  ```

- [x] 3. Implement INI safe write path (backup + atomic + lock)

  **What to do**:
  - Add write API that updates only targeted lines.
  - Create/overwrite backup as `*.ini.bak` before write.
  - Use atomic temp-write + rename strategy.
  - Preserve original BOM state on disk and newline style.

  **Must NOT do**:
  - No direct overwrite without backup.
  - No write outside selected mod root.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `atomic-fs`, `tdd`, `backend-development`
  - **Skills Evaluated but Omitted**:
    - `writing-unit-tests`: covered by `tdd` in this task.

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: 5,9,10
  - **Blocked By**: 2

  **References**:
  - `src-tauri/src/services/operation_lock.rs` - lock semantics and expected error messaging.
  - `.docs/trd.md` (3.3, 3.6) - backup + operation lock requirements.

  **Acceptance Criteria**:
  - [x] Saving creates `.ini.bak` with previous file bytes before overwrite.
  - [x] Write operation is atomic and lock-protected.
  - [x] Round-trip tests show non-target lines remain byte-identical (except expected line edits).

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Save creates backup and updates only target line
    Tool: Bash
    Preconditions: Unit tests for selective line updates exist
    Steps:
      1. Run: cd src-tauri && cargo test ini_write_creates_bak_and_updates_target_line -- --nocapture
      2. Assert backup file existence and exact previous content checksum.
      3. Assert resulting INI changed only at expected line index.
    Expected Result: Pass with verified backup + selective update.
    Evidence: .sisyphus/evidence/task-3-write-happy.txt

  Scenario: Concurrent save blocked by operation lock
    Tool: Bash
    Preconditions: Concurrency test exists
    Steps:
      1. Run: cd src-tauri && cargo test ini_write_lock_contention -- --nocapture
      2. Assert second operation returns "Operation in progress. Please wait.".
    Expected Result: Concurrency safety enforced.
    Evidence: .sisyphus/evidence/task-3-write-negative.txt
  ```

- [x] 4. Align image discovery/paste policy for Epic 6

  **What to do**:
  - Adjust/extend discovery priorities to include explicit `preview_custom` precedence and depth up to Epic 6 target.
  - Keep output format `preview_custom.png` for v1.
  - Add size guard path for pasted image payloads per policy.

  **Must NOT do**:
  - Do not introduce WebP-only storage in v1.

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `backend-development`, `tdd`
  - **Skills Evaluated but Omitted**:
    - `virt-grid`: not relevant to backend image discovery.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: 5,8,10
  - **Blocked By**: 1

  **References**:
  - `src-tauri/src/services/scanner/thumbnail.rs` - existing discovery algorithm and tests.
  - `src-tauri/src/commands/mod_cmds.rs` (`paste_thumbnail`) - current write behavior.
  - `.docs/epic6-previewpanel.md` - target discovery order and clipboard constraints.

  **Acceptance Criteria**:
  - [x] Discovery priority behavior covered by tests.
  - [x] Large-image rejection behavior covered by tests.
  - [x] Existing thumbnail cache behavior remains functional.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Discovery chooses preview_custom first
    Tool: Bash
    Preconditions: Thumbnail tests updated
    Steps:
      1. Run: cd src-tauri && cargo test thumbnail_prefers_preview_custom -- --nocapture
      2. Assert selected path is preview_custom.*.
    Expected Result: Priority order honored.
    Evidence: .sisyphus/evidence/task-4-discovery-happy.txt

  Scenario: Oversized paste rejected
    Tool: Bash
    Preconditions: Negative test exists for payload size guard
    Steps:
      1. Run: cd src-tauri && cargo test paste_thumbnail_rejects_oversize -- --nocapture
      2. Assert error contains "Image too large".
    Expected Result: Rejection path works with no write side effects.
    Evidence: .sisyphus/evidence/task-4-discovery-negative.txt
  ```

- [x] 5. Add/expand Tauri command bridge for details workflows

  **What to do**:
  - Add commands for listing INI files, reading parsed INI model, writing INI edits.
  - Add command for listing preview image set for gallery (ordered).
  - Keep command layer thin; logic in services.

  **Must NOT do**:
  - No business logic bloating in command files.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `tauri-command`, `backend-development`, `tdd`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: 6,7,8,9
  - **Blocked By**: 2,3,4

  **References**:
  - `src-tauri/src/commands/mod_cmds.rs` - existing command conventions and serialization style.
  - `src-tauri/src/lib.rs` - command registration location.
  - `src-tauri/src/commands/mod.rs` - module export pattern.

  **Acceptance Criteria**:
  - [x] New commands registered and callable from frontend.
  - [x] Command tests validate expected success/error envelopes.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Command bridge smoke test
    Tool: Bash
    Preconditions: Command tests added
    Steps:
      1. Run: cd src-tauri && cargo test details_command_bridge_smoke -- --nocapture
      2. Assert list/read/write commands return expected shapes.
    Expected Result: Bridge works for core flows.
    Evidence: .sisyphus/evidence/task-5-commands-happy.txt

  Scenario: Invalid path rejected by command layer
    Tool: Bash
    Preconditions: Negative command test exists
    Steps:
      1. Run: cd src-tauri && cargo test details_command_rejects_path_escape -- --nocapture
      2. Assert error envelope returned and no file write attempted.
    Expected Result: Path traversal/input misuse blocked safely.
    Evidence: .sisyphus/evidence/task-5-commands-negative.txt
  ```

- [x] 6. Build frontend details data layer and query keys

  **What to do**:
  - Add hooks under `src/features/details/hooks/` for mod info, preview images, INI file list, INI document, and save mutations.
  - Define query-key strategy and invalidation rules after save/paste/toggle.
  - Integrate with existing selected item state from `useAppStore`.

  **Must NOT do**:
  - No direct `invoke` calls spread across many UI components.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: `tdd`, `vercel-react-best-practices`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: 7,8,9
  - **Blocked By**: 5

  **References**:
  - `src/hooks/useFolders.ts` - React Query and mutation patterns used today.
  - `src/stores/useAppStore.ts` - selection/layout state integration points.

  **Acceptance Criteria**:
  - [x] Hooks expose typed data + loading/error states.
  - [x] Invalidation updates panel after save and paste operations.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Hook integration unit tests
    Tool: Bash
    Preconditions: Vitest hook tests created
    Steps:
      1. Run: pnpm test src/features/details/hooks
      2. Assert query/mutation tests pass with mocked invoke responses.
    Expected Result: Stable details data layer.
    Evidence: .sisyphus/evidence/task-6-hooks-tests.txt

  Scenario: Mutation error state propagation
    Tool: Bash
    Preconditions: Hook test includes failing invoke mock
    Steps:
      1. Run: pnpm test src/features/details/hooks -- -t "mutation error"
      2. Assert hook exposes error state and does not clear stale data incorrectly.
    Expected Result: Error handling path is deterministic and test-covered.
    Evidence: .sisyphus/evidence/task-6-hooks-negative.txt
  ```

- [ ] 7. Refactor PreviewPanel metadata section (read + debounced autosave)

  **Implementation Progress (2026-02-16)**:
  - PreviewPanel is now wired to selected folder data via details hooks (`useSelectedModPath`, `useSelectedModInfo`).
  - Metadata title/description fields now use 500ms debounced autosave to `update_mod_info` with error toast on failure.
  - Enable toggle now syncs with active folder selected from FolderGrid state.

  **What to do**:
  - Replace hardcoded values with fetched mod info.
  - Implement 500ms debounced autosave for title/description fields.
  - Bind enable/disable toggle to existing toggle logic path.

  **Must NOT do**:
  - No silent save failures; all failures surface toast feedback.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: `frontend-ui-ux`, `tdd`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 8)
  - **Blocks**: 10
  - **Blocked By**: 6

  **References**:
  - `src/features/details/PreviewPanel.tsx` - current stub structure to evolve.
  - `src/features/explorer/FolderCard.tsx` - toggle/favorite interaction style.
  - `.docs/.testcase/TC-Epic-06-PreviewPanel.md` (TC-6.1-01, NC-6.1-01, NC-6.1-02).

  **Acceptance Criteria**:
  - [ ] Debounced autosave verified for description edits.
  - [ ] Corrupt info path handled with fallback + warning.
  - [ ] Permission denied path shows actionable error toast.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Debounced metadata autosave
    Tool: Playwright
    Preconditions: Dev app running; details panel bound to test mod
    Steps:
      1. Navigate to app dashboard -> mods view.
      2. Select test mod card.
      3. Fill description textarea with "Epic6 test save".
      4. Wait 700ms.
      5. Assert save success toast appears.
      6. Re-open same mod and assert textarea value persists.
      7. Screenshot: .sisyphus/evidence/task-7-autosave-happy.png
    Expected Result: Debounced save persists to info.json.
    Evidence: .sisyphus/evidence/task-7-autosave-happy.png

  Scenario: Read-only info.json save blocked
    Tool: Playwright
    Preconditions: info.json file attribute read-only for test mod
    Steps:
      1. Edit description and wait debounce.
      2. Assert error toast contains "Cannot save metadata".
      3. Assert field remains editable but unsaved indicator remains.
      4. Screenshot: .sisyphus/evidence/task-7-autosave-negative.png
    Expected Result: Graceful failure with no crash/data loss.
    Evidence: .sisyphus/evidence/task-7-autosave-negative.png
  ```

- [x] 8. Implement gallery slider, lazy load policy, and paste integration

  **Implementation Progress (2026-02-16)**:
  - Preview gallery now reads ordered images from `useSelectedPreviewImages` and supports prev/next navigation.
  - Paste action is wired through `usePastePreviewImage` and refreshes image list after successful mutation.
  - Lazy-load behavior is enforced as current ±1 image window with placeholders for non-window images.
  - Gallery uses right-click context menu actions (paste/import/remove/clear) with confirmation on destructive actions.
  - Broken image fallback placeholder is displayed on load failure.
  - Backend discovery updated to root `preview*` first, fallback deep-scan depth 5 when root preview missing.
  - Backend preview naming updated to `preview_[objectname].png` with first-available numeric suffix.

  **What to do**:
  - Replace placeholder carousel with real image collection.
  - Enforce current ±1 image load policy; others placeholder.
  - Wire paste action to command flow and refresh gallery immediately.

  **Must NOT do**:
  - No eager load of all images for large sets.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: `frontend-ui-ux`, `virt-grid`, `tdd`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 7)
  - **Blocks**: 10
  - **Blocked By**: 6

  **References**:
  - `src/features/details/PreviewPanel.tsx` - current gallery toolbar location.
  - `src/hooks/useFolders.ts` (`usePasteThumbnail`) - existing mutation entrypoint.
  - `.docs/epic6-previewpanel.md` (US-6.2, lazy loading requirement).

  **Acceptance Criteria**:
  - [x] Slider arrows and indicators behave correctly for N>1.
  - [x] Lazy load window verified via utility tests (`shouldLoadGalleryImage`) and placeholder rendering for non-window images.
  - [x] Paste refreshes list and uses backend `paste_thumbnail` flow.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Slider lazy-load behavior with 20 images
    Tool: Playwright
    Preconditions: Test mod contains 20 preview images
    Steps:
      1. Open details panel for test mod.
      2. Assert only current image and adjacent nodes have loaded img src.
      3. Click next arrow twice.
      4. Assert next adjacent images load and previous far images remain placeholder.
      5. Screenshot: .sisyphus/evidence/task-8-slider-happy.png
    Expected Result: Current ±1 loading policy respected.
    Evidence: .sisyphus/evidence/task-8-slider-happy.png

  Scenario: Clipboard paste updates gallery
    Tool: Playwright
    Preconditions: Clipboard contains image data; panel focused
    Steps:
      1. Trigger paste action button.
      2. Wait for success toast.
      3. Assert first gallery item path contains preview_custom.png.
      4. Screenshot: .sisyphus/evidence/task-8-paste-happy.png
    Expected Result: New image persisted and gallery refreshed.
    Evidence: .sisyphus/evidence/task-8-paste-happy.png
  ```

- [x] 9. Deliver INI editor UI with dirty-state guard and validation

  **Implementation Progress (2026-02-16)**:
  - INI documents are loaded from `list_mod_ini_files` + `read_mod_ini` and presented as grouped sections (no file dropdown).
  - Structured parser output is rendered as editable variable/key/back rows with line-number context.
  - Save flow calls `write_mod_ini` with selective line updates and line-level validation errors.
  - Dirty-state guard modal added for file/mod navigation with Save / Discard / Cancel actions.
  - UI now exposes 2 editor tabs: `Key Bind` and `Information`.
  - Key Bind tab groups editable fields by `source_file` and `section_name` with expand/collapse sections.
  - Information tab is read-only and shows variable ranges plus `$active` occurrences (file/section).
  - UI compacted using DaisyUI tab variants and line-index labels removed from editor/info presentation.

  **What to do**:
  - Build INI file selector dropdown.
  - Render keybinding and variable controls from parsed model.
  - Add save flow with validation and error line feedback.
  - Add unsaved-changes modal on mod/file navigation.

  **Must NOT do**:
  - No free-form full-file rewrite editor in v1.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `frontend-ui-ux`, `tdd`, `writing-unit-tests`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: 10
  - **Blocked By**: 3,5,6

  **References**:
  - `.docs/epic6-previewpanel.md` (US-6.3 + unsaved guard checklist).
  - `.docs/.testcase/TC-Epic-06-PreviewPanel.md` (TC-6.3-01..03, TC-6.4-01, NC-6.3-\*).

  **Acceptance Criteria**:
  - [x] Keybinding and variable fields render from parser output.
  - [x] Invalid value blocks save with precise feedback.
  - [x] Unsaved changes modal supports Save / Discard / Cancel paths.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Edit variable and save with backup
    Tool: Playwright + Bash
    Preconditions: Test mod includes editable INI
    Steps:
      1. Open INI tab and choose target file from dropdown.
      2. Change numeric variable from 0 to 1.
      3. Click Save.
      4. Assert success toast.
      5. Run filesystem assertion command to confirm .ini.bak exists.
      6. Screenshot: .sisyphus/evidence/task-9-ini-save-happy.png
    Expected Result: INI updated, backup created before write.
    Evidence: .sisyphus/evidence/task-9-ini-save-happy.png

  Scenario: Unsaved changes navigation guard
    Tool: Playwright
    Preconditions: INI form dirty
    Steps:
      1. Edit any variable without saving.
      2. Click another mod card.
      3. Assert modal text includes "Unsaved changes" and options Save/Discard/Cancel.
      4. Click Cancel and assert still on same mod.
      5. Screenshot: .sisyphus/evidence/task-9-dirty-guard.png
    Expected Result: Navigation blocked until explicit decision.
    Evidence: .sisyphus/evidence/task-9-dirty-guard.png
  ```

- [x] 10. Full TC-mapped verification, performance gate, and evidence pack

  **What to do**:
  - Execute frontend and backend suites for Epic 6 touchpoints.
  - Run targeted performance checks for TM-6.01/6.02/6.03.
  - Assemble evidence index under `.sisyphus/evidence/`.

  **Must NOT do**:
  - Do not mark complete if any critical TC remains unmapped/unverified.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `verification-before-completion`, `writing-unit-tests`, `playwright`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Final wave
  - **Blocks**: None
  - **Blocked By**: 7,8,9

  **References**:
  - `.docs/.testcase/TC-Epic-06-PreviewPanel.md` - all final pass/fail criteria.
  - `vite.config.ts` and `package.json` - frontend test command conventions.
  - `src-tauri/Cargo.toml` - backend test tooling context.

  **Acceptance Criteria**:
  - [ ] `pnpm test` passes for affected frontend suites.
  - [ ] `cd src-tauri && cargo test` passes for affected backend suites.
  - [ ] Evidence files exist for every scenario in Tasks 7-9.
  - [ ] Performance checks meet thresholds or include explicit risk note and follow-up task.

  **Agent-Executed QA Scenarios**:

  ```text
  Scenario: Final regression pack
    Tool: Bash
    Preconditions: All implementation tasks complete
    Steps:
      1. Run: pnpm test
      2. Run: cd src-tauri && cargo test
      3. Assert both exit code 0.
      4. Verify evidence directory contains task-7/8/9 screenshots and logs.
    Expected Result: Epic 6 verification gate passes.
    Evidence: .sisyphus/evidence/task-10-regression.txt

  Scenario: Performance threshold breach detection
    Tool: Bash
    Preconditions: Perf check scripts for parse/render/lazy-load are available
    Steps:
      1. Run perf checks for TM-6.01/TM-6.02/TM-6.03.
      2. Assert failures produce explicit non-zero exit and report file.
    Expected Result: Threshold breaches are visible and block completion.
    Evidence: .sisyphus/evidence/task-10-performance-negative.txt
  ```

---

## Commit Strategy

| After Task | Message                                                               | Files                                                           | Verification                              |
| ---------- | --------------------------------------------------------------------- | --------------------------------------------------------------- | ----------------------------------------- |
| 2-3        | `feat(epic6): add lossless ini read-write service with safety guards` | `src-tauri/src/services/file_ops/*`, `src-tauri/src/commands/*` | `cd src-tauri && cargo test`              |
| 4-5        | `feat(epic6): add preview image and details command bridge`           | backend services/commands + registration                        | `cd src-tauri && cargo test`              |
| 6-9        | `feat(epic6): wire preview panel data, gallery, and ini editor flows` | `src/features/details/**`, hooks/types                          | `pnpm test`                               |
| 10         | `test(epic6): add tc-mapped coverage and verification evidence`       | tests/evidence docs                                             | `pnpm test && cd src-tauri && cargo test` |

---

## Success Criteria

### Verification Commands

```bash
pnpm test
# Expected: frontend tests pass

cd src-tauri && cargo test
# Expected: backend tests pass
```

### Final Checklist

- [ ] All Must Have items delivered.
- [ ] All Must NOT Have guardrails respected.
- [ ] TC/NC/EC/TM/DI coverage is traceable and evidenced.
- [ ] No known data corruption path remains in INI/info/image flows.
