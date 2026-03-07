# Test Cases: Bulk Operations & Selection (req-14)

## A. Requirement Summary

- **Feature Goal**: Manage massive mod library arrays enabling structured multi-select bounds executing parallel-friendly atomic backend actions securely isolating exactly resolving conflicts natively without destroying array mapping structures partially.
- **User Stories**:
 - US-14.1: Multi-Select Mods
 - US-14.2: Bulk Action Bar
 - US-14.3: Bulk Move
- **Success Criteria**:
 - Selection mapping visually completes (Checkbox/Shift-Click) <50ms.
 - Action Bar logic animates <100ms.
 - Streaming operations logically maps progression updating arrays.
 - Partial bounds failure maps distinct contextual errors structurally bypassing mapping mechanical whole-batch abort structures.
- **Main Risks**: Destructive arrays mapped incorrectly deleting entirely wrong components mechanically. Race conditions executing parallel structures violently matching identical mutex blocks internally. 1 in 100 object moving locks terminating the entire batch operation halfway.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-14-bulk-operations.md`

- AC-14.1.1, AC-14.1.2 → TC-14-01
- AC-14.1.3 → TC-14-02
- AC-14.1.4, AC-14.1.5 → TC-14-03
- AC-14.2.1, AC-14.2.2 → TC-14-04
- AC-14.2.3 → TC-14-05
- AC-14.3.1, AC-14.3.2 → TC-14-06
- AC-14.3.3 → TC-14-07
- AC-14.3.4 → TC-14-08

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ------------------------------------------------ | -------- | -------- | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- | -------------------- |
| TC-14-01 | Basic Selection & Action Bar bounds | Positive | High |`Valid entries` | 1. Grid populated visually.<br>2. Hover item mapping.<br>3. Click Checkbox. | State natively saves exactly matching target strings.`BulkActionBar` logically maps interface bounding context rendering operations accurately <100ms. | S2 | AC-14.1.1, AC-14.1.2 |
| TC-14-02 | Rapid Range Selection logical bounds | Positive | High |`>50 elements` | 1. Click specifically explicit target.<br>2. Hold`Shift` clicking explicit target bounds 50 lines mapping. | System instantaneously executes intersection array mappings logically calculating precisely structural boundary elements selecting <50ms. | S2 | AC-14.1.3 |
| TC-14-03 | Protecting Active background processing loops | Negative | High |`Processing Target` | 1. Click targeted explicit block attempting selection mapping constraints.<br>2. Shift paths mapping externally. | Selection denies mapping avoiding state traps visually resetting accurately handling path navigation drops. | S2 | AC-14.1.4, AC-14.1.5 |
| TC-14-04 | Bulk Bar UI routing commands | Positive | Med |`Multiple Targets` | 1. Verify buttons function.<br>2. Trigger explicit "Deselect All". | Toolbar accurately executes parameters exactly mapping text matching exact components hiding interface exactly <100ms. | S3 | AC-14.2.1, AC-14.2.2 |
| TC-14-05 | Bulk Failure logic isolation constraints | Edge | High |`Explicit backend trap` | 1. Simulate mechanical total operation explicit failure executing internal bounds. | Component mechanically resets exact state mappings fully presenting exactly localized Toast feedback matching "failed - {X} errors" without locking logic permanently loading state. | S1 | AC-14.2.3 |
| TC-14-06 | Multi Move Progress streaming events | Positive | High |`50 Targets` | 1. Select 50 explicit objects.<br>2. Click logically explicit "Move to Object" bound.<br>3. Track Frontend context arrays. | Backend streams JSON progression exact variables matching 1-update/sec bounding cache visually resolving exact paths. | S2 | AC-14.3.1, AC-14.3.2 |
| TC-14-07 | Partial Batch Conflicts mapping | Negative | Critical |`Duplicated Names` | 1. Force structurally mapped directory move collision payload internally.<br>2. Observe Toast feedback. | Backend pauses specific execution bounding exact structural dialog effectively bypassing leaving other independent payloads operating without batch abort logic. | S1 | AC-14.3.3 |
| TC-14-08 | Mutex Lock Batch operations guarding state | Edge | Med |`2 Concurrent ops` | 1. Initiate Active specific batch array executing internally.<br>2. Attempt natively starting a totally explicit second Bulk action. | Target UI denies matching mechanical buttons enforcing single threading precisely against mapped`OperationLock` avoiding race exceptions. | S1 | AC-14.3.4 |
| TC-14-09 | [Implied] Watcher Suppression scaling explicitly | Implied | Critical |`Valid batch` | 1. Execute explicit macro rename bounds mapping structs.<br>2. Assert File Watcher explicit system natively mapping structs internally. | Array appends all targeted batch entries within explicitly massive overarching suppression set natively dropping precisely 0 recursive update exceptions. | S2 | N/A |

## D. Missing / Implied Test Areas

- **Ctrl-Click Support**: Does holding`Ctrl` logic toggle individual items independently similar to native OS bounds?
- **Sorting effects on Shift-Click**: If I click item A, sort by modified, then shift-click item B, it MUST accurately map the array _based on the current sorted display list_, not the absolute default JSON string array precisely.

## E. Open Questions / Gaps

- "Progress bar for batches <5 items - single toast suffices". This handles performance. Does the UI just show a spinner in the Bulk Bar instead of a progress slider natively?

## F. Automation Candidates

- **TC-14-02 (Immediate Shift-Selection arrays)**: Cypress tests strictly evaluating DOM list node attributes acquiring entirely bounded lists inside UI elements.
- **TC-14-07 (Partial Batch Collisions natively)**: Native Rust Backend isolation explicitly ensuring structural arrays handle isolated result vectors natively resolving errors.

## G. Test Environment Setup

- **Preconditions**: UI state engines tracking huge Boolean`Set<String>` matrices mapping physical arrays without degrading rendering pipeline.
- **Context Injection**: 200+ mock instances mapping specific naming collisions ensuring exactly structural multi-threading logic aborts.

## H. Cross-Epic E2E Scenarios

- **E2E-14-01 (Bulk Collision Streamed UI)**: Multi-selection DOM rendering constructs a Set array (Epic 14). User triggers Bulk Rename (Epic 13) resulting in an intentional disk collision. The error stream populates the mapping parameters (Epic 36) and generates a Conflict Resolution dialog.
