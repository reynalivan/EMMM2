# Epic 45: Import Object Library Readiness Gate

## 1. Executive Summary

- **Problem**: The current Object Library identification prompt runs before an import batch is created and before its sources are analyzed. It scans the entire library, so the prompt can be unrelated to the incoming mods and cannot improve an already computed review in a controlled way.
- **Solution**: Introduce a backend-owned, pre-review readiness gate that runs after source analysis but before `Review Import Matches` is first shown. The gate recommends clarification only for existing objects relevant to the analyzed batch. Applying clarification refreshes the batch's matches without extracting or analyzing the source again.
- **Outcome**: Users see a clean review flow, library clarification remains optional, and the Match Wizard benefits from confirmed Object Library identity without blocking import.

## 2. Definitions

| Term | Definition |
| --- | --- |
| **Source analysis** | Extraction, source inspection, fingerprinting, category evidence, and initial destination matching for an import batch. |
| **Library readiness gate** | A read-only check between source analysis and the first Match Wizard review. It is not a filesystem mutation preflight. |
| **Relevant object** | An existing Object Library entry that is a destination candidate for at least one item in the analyzed batch. |
| **Clarification** | User review of an existing object's category and canonical identity through the Object Classification Wizard. |
| **Review started** | The point immediately before `Review Import Matches` is first rendered for a batch. |
| **Match refresh** | Recalculation of canonical and destination suggestions using the updated Object Library, without extraction or source staging work. |

## 3. Scope and Non-Goals

### In Scope

- Run readiness after source analysis and before the first Match Wizard review.
- Restrict readiness candidates to objects relevant to the analyzed import batch.
- Show the prompt only when at least one high- or medium-confidence identification is found.
- Let users review the library, defer for 24 hours, or cancel clarification and continue import.
- Refresh all import matches in one backend operation after classifications are applied.
- Persist whether review has started so resumed batches do not repeatedly show the gate.
- Keep all labels localized.

### Non-Goals

- Blocking import until Object Library clarification is completed.
- Moving or renaming existing object folders during readiness preview.
- Re-extracting archives after clarification.
- Replacing filesystem mutation preflight or Disk Reconcile readiness checks.
- Automatically accepting a canonical identification without user review.

## 4. Required End-to-End Flow

```text
User selects source
        |
        v
Create or resume import batch
        |
        v
Analyze source
- extract archive when needed
- inspect folder and files
- fingerprint source
- calculate category evidence
- calculate initial destination matches
        |
        v
Has Review Import Matches already opened for this batch?
        |
        +-- Yes --> Review Import Matches
        |
        +-- No
              |
              v
     Is this a specific-target import?
              |
        +-----+-----+
        |           |
       Yes          No
        |           |
        v           v
 Review Matches   Check Object Library readiness
                          |
                  +-------+--------+
                  |                |
             No match       X identifications found
                  |                |
                  v                v
           Review Matches     Pre-review prompt
                               |          |
                          Remind later   Review library
                               |          |
                               v          v
                        Review Matches  Classification Wizard
                                            |
                                   +--------+--------+
                                   |                 |
                                Cancel             Apply
                                   |                 |
                                   v                 v
                            Review Matches    Refresh batch matches
                                                     |
                                                     v
                                             Review Import Matches
```

## 5. Gate Rules

1. The gate runs only after source analysis reaches a reviewable batch state.
2. `Review Import Matches` must not render while readiness is being checked or its prompt is open.
3. A prompt requires at least one relevant canonical identification with `high` or `medium` confidence.
4. High-confidence results still require user review; readiness never writes classifications automatically.
5. Specific-target imports skip the gate because their physical destination is already explicit.
6. A resumed batch skips the gate when `review_started_at` is already set.
7. `Remind later` opens review immediately and suppresses the prompt for new batches of the same game for 24 hours.
8. Cancelling the Object Classification Wizard opens Match Wizard with the existing initial results.
9. Readiness failures are fail-open: log the diagnostic and continue to Match Wizard.
10. A single object referenced by multiple import items appears once in the clarification list.

## 6. Backend Ownership

React owns presentation and user choice. Rust owns candidate relevance, classification preview, confidence filtering, batch refresh, and persisted review state.

### 6.1 Readiness Preview Command

```rust
preview_import_library_readiness(
    batch_id: String,
) -> Result<ImportLibraryReadiness, AppError>
```

```rust
struct ImportLibraryReadiness {
    batch_id: String,
    items: Vec<ObjectClassificationPreviewItem>,
    high_count: u32,
    medium_count: u32,
}
```

The application service must:

1. Validate that the batch exists and belongs to a configured game.
2. Reject preview while source analysis is incomplete.
3. Derive candidate object IDs from the batch's destination matching evidence.
4. Exclude objects already carrying a confirmed `matched_entry_key`.
5. Deduplicate object IDs before inspection.
6. Reuse the existing category and canonical classification engines.
7. Return only high- and medium-confidence identifications, sorted by confidence descending.
8. Perform no filesystem or database mutation.

### 6.2 Batch Match Refresh Command

```rust
refresh_import_batch_matches(
    batch_id: String,
) -> Result<ImportBatch, AppError>
```

The command must:

- Reload the Master DB after Object Library aliases or identities change.
- Recompute canonical and destination suggestions for every non-terminal item.
- Preserve staged source paths, fingerprints, source previews, and planned names.
- Avoid archive extraction and full source analysis.
- Reset only automatic decisions on a batch that has not started review.
- Execute as one backend orchestration call instead of one IPC request per item.

### 6.3 Review Persistence

Add the nullable UTC timestamp below to `import_batches`:

```text
review_started_at
```

Expose:

```rust
mark_import_batch_review_started(
    batch_id: String,
) -> Result<(), AppError>
```

The command is idempotent. The frontend calls it immediately before first rendering Match Wizard. A crash before this point may show readiness again; a crash after this point resumes directly into review.

## 7. Frontend State Machine

`ImportBatchWizardHost` must use an explicit phase rather than treating `batch !== null` as permission to render Match Wizard.

```ts
type ImportWizardPhase =
  | 'idle'
  | 'analyzing'
  | 'checking_library'
  | 'library_prompt'
  | 'classifying_library'
  | 'refreshing_matches'
  | 'reviewing';
```

| Phase | Visible surface |
| --- | --- |
| `idle` | No import modal, except the resumable-batch action when available. |
| `analyzing` | Extraction and source-analysis progress. |
| `checking_library` | Compact loading state; Match Wizard remains hidden. |
| `library_prompt` | `X identifications found` pre-review prompt. |
| `classifying_library` | Existing Object Classification Wizard. |
| `refreshing_matches` | Compact loading state while Rust refreshes the batch. |
| `reviewing` | `Review Import Matches`. |

The classification launcher must always resolve its completion callback as either `applied` or `cancelled`. Both outcomes continue the held import batch; only `applied` triggers match refresh.

## 8. Reminder Behavior

- Reminder scope is the active game, not a source path or object.
- Default duration is 24 hours.
- Reminder persistence may remain frontend-local because it is advisory and non-critical.
- Reminder state is checked after analysis but before requesting readiness preview to avoid unnecessary work.
- Deferring marks the current batch as review-started before opening Match Wizard.

## 9. Error and Concurrency Behavior

- A readiness preview error must not cancel or delete the import batch.
- A stale classification fingerprint stays actionable in the Classification Wizard and does not corrupt the held import.
- If the Object Library changes after readiness preview, classification apply must retain its existing fingerprint and canonical-entry validation.
- If match refresh fails after classification was saved, show a localized warning and allow retry; do not re-run extraction.
- Duplicate launch events for the same batch must converge on one host phase and one readiness request.
- Closing the application during the prompt leaves the batch resumable.

## 10. Implementation Slices

1. Add readiness DTO, application service, Tauri command, and generated bindings.
2. Add `review_started_at`, idempotent repository update, and mark-review command.
3. Add backend batch-level match refresh using existing matching services.
4. Refactor `ImportBatchWizardHost` to the explicit phase state machine.
5. Move the existing whole-library discovery logic out of React.
6. Ensure classification cancel and apply both resume the held import flow.
7. Add localized checking, refreshing, prompt, warning, and retry labels.
8. Add focused frontend and backend tests for gate transitions and candidate relevance.

## 11. Acceptance Criteria

| ID | Criteria |
| --- | --- |
| AC-45.01 | A new analyzed batch with relevant identifications shows the readiness prompt before Match Wizard. |
| AC-45.02 | An identification elsewhere in Object Library that is unrelated to the batch does not trigger the prompt. |
| AC-45.03 | No high- or medium-confidence result opens Match Wizard directly. |
| AC-45.04 | `Remind later` opens Match Wizard and suppresses prompts for new batches of that game for 24 hours. |
| AC-45.05 | Cancelling clarification opens Match Wizard with the initial matches. |
| AC-45.06 | Applying clarification refreshes matches without extracting the source again. |
| AC-45.07 | A resumed batch with no `review_started_at` executes the gate. |
| AC-45.08 | A resumed batch with `review_started_at` opens Match Wizard directly. |
| AC-45.09 | Specific-target import skips readiness and opens Match Wizard after analysis. |
| AC-45.10 | A readiness backend failure does not block import review. |
| AC-45.11 | Multiple source items referencing the same object produce one clarification item. |
| AC-45.12 | The frontend does not determine relevance or confidence tiers from the full Object Library. |
| AC-45.13 | Readiness preview performs no filesystem or database mutation. |
| AC-45.14 | All visible labels and errors use i18n resources. |

## 12. Dependencies

- **Depends On**:
  - Epic 23: Mod Import.
  - Epic 26: Deep Matcher.
  - Epic 27: Object Library database synchronization.
  - Epic 44: Shared smart-import entry points.
- **Reuses**:
  - `ObjectClassificationWizardHost`.
  - Object classification preview and apply services.
  - Import batch source analysis and suggestion refresh services.
