# Test Cases: Smart Filters & Sorting (req-08)

## A. Requirement Summary

- **Feature Goal**: High-performance, memory-safe data filtering and searching for ObjectList objects utilizing offloaded web-workers for fuzzy queries, DB-side limits for Safe Mode enforcement, and boolean toggle flags.
- **User Stories**:
 - US-08.1: ObjectList Sorting
 - US-08.2: Fuzzy Text Searching
 - US-08.3: Empty / Uncategorized Toggles
 - US-08.4: Safe Mode Content Filter
- **Success Criteria**:
 - Search/Sort resolves instantly via keystroke debounce ≤100ms.
 - Fuzzy logic algorithm meets minimum 0.75 partial matching thresholds avoiding hard absolute type string needs.
 - Safe mode natively eradicates flagged object payloads BEFORE transmitting them physically over API/IPC avoiding hidden DOM traces entirely (≤100ms).
 - Toggles (Empty/Uncategorized) function in list rendering maps.
- **Main Risks**: Main thread locking from brute-force string iterations on 10,000 array payloads. 'Safe Mode' implementing exclusively via CSS`display:none` accidentally exposing sensitive strings logically inside browser DevTools payload inspectors.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-08-smart-filters.md`

- AC-08.1.1, AC-08.1.2, AC-08.1.3 → TC-08-01
- AC-08.1.4 → TC-08-02
- AC-08.1.5 → TC-08-03
- AC-08.2.1, AC-08.2.2 → TC-08-04
- AC-08.2.3 → TC-08-05
- AC-08.2.4 → TC-08-06
- AC-08.3.1, AC-08.3.2 → TC-08-07
- AC-08.3.3 → TC-08-08
- AC-08.3.4 → TC-08-09
- AC-08.4.1, AC-08.4.2 → TC-08-10
- AC-08.4.3 → TC-08-11
- AC-08.4.4 → TC-08-12

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ------------------------------------ | -------- | -------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------- |
| TC-08-01 | Sorting Execution Persisted | Positive | High |`DB Objects` | 1. Open ObjectList.<br>2. Toggle between 'A-Z' and 'Active First' via settings dropdown.<br>3. Reload App. | Items restructure. 'Active First' forces`enabled_count > 0` directly to header section bounds. Behavior saved natively via`localStorage`. | S3 | AC-08.1.1, AC-08.1.2, AC-08.1.3 |
| TC-08-02 | Stale or Corrupted sort enums | Negative | Low |`sidebarSort=EVIL` | 1. Close application.<br>2. Manipulate browser`localStorage` setting`sidebarSort` to`EVIL`.<br>3. Boot Application. | Fallback defaults intelligently to 'A-Z' enum logic gracefully without crashing React render cycles. | S4 | AC-08.1.4 |
| TC-08-03 | Sorting resolution ties | Edge | Med |`3 active equivalent IDs` | 1. Ensure 3 objects share exact identical active`enabled_count`.<br>2. Activate 'Active First' sort constraint. | Ties logically map fallback parameters defaulting alphabetically providing structurally repeatable stability across multiple identical runs. | S3 | AC-08.1.5 |
| TC-08-04 | Fuzzy Query Performance/Logic | Positive | High |`Input: "hutao"` | 1. Assert DB holds target string "Hu Tao" among 1000 items.<br>2. Focus Search Input phrase and type`hutao` rapidly. | Search offloads calculations. Bypasses main-thread latency scoring matched strings via parameters ≥ 0.75 without visible input lag (≤100ms). | S2 | AC-08.2.1, AC-08.2.2 |
| TC-08-05 | Unicode / Error text queries | Negative | Low |`Input: "$#@!"` | 1. Focus Search.<br>2. Enter absurd unicode sequences / symbols. | Search avoids exploding/returning undefined maps. Soft responds natively 0 results without breaking standard worker pipeline. | S4 | AC-08.2.3 |
| TC-08-06 | Scrapping Async Search worker | Edge | High |`Fast Input sequence` | 1. Type long command string.<br>2. Immediately hit`Backspace` or`Escape` clear block before UI paint sequence maps back to worker resolution. | Original standard global list fully replaces stale specific job results preserving structural accuracy ≤50ms. | S2 | AC-08.2.4 |
| TC-08-07 | Toggling visibility booleans | Positive | Med |`Empty row / Unknown` | 1. Focus ObjectList Filters.<br>2. Toggle "Hide Empty".<br>3. Toggle "Show Uncategorized". | Null parameters natively stripped/hidden. Unmatched schema folders attached securely underneath active UI lists. | S3 | AC-08.3.1, AC-08.3.2 |
| TC-08-08 | Hiding actively checked elements | Negative | Med |`Object folder_count 0` | 1. Click specifically an explicitly 'Empty' target.<br>2. Enable "Hide Empty" via Filters. | Object vanishes.`selectedObjectId` forces state clear wiping orphaned references avoiding ghost grid renderings. | S3 | AC-08.3.3 |
| TC-08-09 | Reactive un-hiding events | Edge | Med |`Background task move` | 1. "Hide Empty" active initially.<br>2. Move mod natively into functionally empty parent target boundary utilizing Grid folder structures. | Invalidation instantly bypasses boolean trap un-collapsing node mapping appropriately into UI list logic. | S3 | AC-08.3.4 |
| TC-08-10 | Safe Mode Hard Limits | Positive | Critical |`Explicit NSFW DB record` | 1. Configure explicitly Risky Mod Data.<br>2. Enable Global Safe Mode parameters structurally via main menu bar exclusively. | Payload never traverses local IPC sequence boundary. Elements fully purged directly via deep SQL query clauses totally. | S1 | AC-08.4.1, AC-08.4.2 |
| TC-08-11 | Explicit boolean overrides heuristic | Negative | High |`Name: "Cat", is_safe: false` | 1. Inject non-risky named parameter holding explicit risky explicit boolean override physically.<br>2. Verify via DB mapping directly. | Native override strictly rules all heuristics securely enforcing absolute suppression map. | S1 | AC-08.4.3 |
| TC-08-12 | Dynamic Privacy Deselection | Edge | Critical |`Selected Risky UI` | 1. Click targeted explicitly risky module.<br>2. Fire Global Safe Mode macro. | Screen blanks specific Grid + Preview mappings instantaneously. Clears pointer ID. Restricts visibility globally ≤100ms. | S1 | AC-08.4.4 |
| TC-08-13 | [Implied] Case Insensitivity | Implied | Low |`Objects "apple" "Apple"` | 1. Inject Mixed-case target labels.<br>2. Sort "A-Z".<br>3. View sorted items. | sorts regardless of character casing to prevent ASCII order separation. | S4 | N/A |

## D. Missing / Implied Test Areas

- **Resetting Search**: Does the search box include an`x` or clear button for QoL, and does using it accurately fire the same 'Clear pending requests' logic as TC-08-06?
- **Web Worker Context**: When Game Context changes entirely while a fuzzy search web worker operates, the worker needs to be reset or ignored, preventing 'Genshin' results flashing on 'WuWa' objectlist views.

## E. Open Questions / Gaps

- Does`Show Uncategorized` objects count get calculated within`Active First` sort algorithms identically to core Schema groups?

## F. Automation Candidates

- **TC-08-10 (IPC Payload Privacy)**: Cypress API-interception test. Inspect the underlying IPC JSON`response` stream verifying risky data properties distinctly do not exist structurally outside of Rust execution spaces when`safeMode` parameter queries.
- **TC-08-04 (Fuzzy worker integration)**: Execute explicit React DOM query sequences matching input characters asserting specific item returns structurally (e.g. testing input`"raid"` strictly returns`[Raiden Shogun]`).

## G. Test Environment Setup

- **Preconditions**: Broad database containing exactly explicit diverse strings ("Hutao", "Hu Tao", "Apple", "apple") combining 10+ explicit strictly assigned 'NSFW' labels.
- **Worker Configuration**: Offload fuzzy algorithms running parallel contexts simulating CPU stress validating Web Worker thread independence.

## H. Cross-Epic E2E Scenarios

- **E2E-08-01 (Privacy Blur Lock)**: User asserts "Hide Empty" AND "Safe Mode" (TC-08-07, TC-08-10), then searches global strings. Validates that the Preview Card (Epic 16) resets and respects the privacy blur settings for NSFW content.
