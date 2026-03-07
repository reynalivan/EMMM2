# Test Cases: Object CRUD Operations (req-10)

## A. Requirement Summary

- **Feature Goal**: Native internal database record controls natively allowing manual curation arrays, correcting system mapping issues without corrupting physical disk hierarchies structurally through full CRUD operations mapping Objects.
- **User Stories**:
 - US-10.1: Create Custom Object
 - US-10.2: Edit Object Properties
 - US-10.3: Delete Object
 - US-10.4: Pin Object
- **Success Criteria**:
 - Object manipulation interactions map to database queries completing (≤300ms creation, ≤200ms optimistic UI edit).
 - Absolutely impossible to delete objects inherently retaining`folder` FK mappings securing relationships.
 - Pin boolean mutates category sorting hierarchy exclusively (≤100ms ui output).
 - Rate-limit execution triggers (button debouncing / UNIQUE clauses) block parallel spam writes.
- **Main Risks**: Deleting connected structures causing SQLite FK enforcement exceptions crashing logic states. Submitting maliciously modified schema IDs outside parameter constraints into Backend SQL transactions.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-10-object-crud.md`

- AC-10.1.1, AC-10.1.2 → TC-10-01
- AC-10.1.3 → TC-10-02
- AC-10.1.4 → TC-10-03
- AC-10.2.1, AC-10.2.2 → TC-10-04
- AC-10.2.3 → TC-10-05
- AC-10.2.4 → TC-10-06
- AC-10.3.1 → TC-10-07
- AC-10.3.2, AC-10.3.3 → TC-10-08
- AC-10.4.1, AC-10.4.2 → TC-10-09
- AC-10.4.3 → TC-10-10
- AC-10.4.4 → TC-10-11

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | -------------------------------------- | -------- | -------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-10-01 | Successful UI Object Creation | Positive | High |`Standard Valid Data` | 1. Click "Add Object" in ObjectList.<br>2. Input string identifier.<br>3. Select valid categorization bounds.<br>4. Submit UI module. | Row propagates structurally mapping database queries effectively ≤300ms. Capable of receiving Drop payload target instantly. | S2 | AC-10.1.1, AC-10.1.2 |
| TC-10-02 | Handling Name Collisions Natively | Negative | High |`Existing Case-Mismatch Name` (`hu tao` vs`Hu Tao`) | 1. Open Object Creation modal.<br>2. Submit exactly matching alias string targeting existing DB struct.<br>3. Inspect UI. | Backend constraint traps exception. Triggers inline warning feedback safely preventing dual-key insertion routines. | S2 | AC-10.1.3 |
| TC-10-03 | Form Button Debouncing Logic | Edge | Med |`Standard Data` | 1. Fill Object Form.<br>2. Execute furious clicking event mapping submission targets >10 times immediately. | First interaction suppresses additional event listener inputs exclusively yielding exactly 1 unique dataset creation event block securely bounding memory. | S3 | AC-10.1.4 |
| TC-10-04 | Core Category + Name mutating | Positive | High |`Valid Context Shift` | 1. Right click Object -> Edit.<br>2. Edit Category dropdown parameter.<br>3. Alter Display name title string.<br>4. Submit. | Database executes parameters mapping strictly modifying list sorting locations optimizing frontend UI cache instantly (≤200ms). | S2 | AC-10.2.1, AC-10.2.2 |
| TC-10-05 | IPC Schema Override Exploit | Negative | High |`Tampered category string` | 1. Fire raw IPC`update_object` containing category string explicitly NOT encoded in memory Schema validation blocks. | Server bounds reject payload execution safely throwing distinct structurally typed validation response trapping memory modifications. | S1 | AC-10.2.3 |
| TC-10-06 | Duplicate Context Warning Trap | Edge | Med |`String targeting another object entity directly` | 1. Open Object Editor.<br>2. Commit name adjustment to already mapped phrase.<br>3. Review Screen. | Software issues UI explicit confusion warning overlay without breaking database mechanics. | S3 | AC-10.2.4 |
| TC-10-07 | Clear Folder Parameter Delete | Positive | High |`Explicit empty entity constraint` | 1. Right click purely EMPTY object.<br>2. Run target component delete call.<br>3. Check UI and DB. | Database checks folder mappings returning empty array structurally dropping internal configuration natively erasing element. | S1 | AC-10.3.1 |
| TC-10-08 | Enforced FK Blocking Deletes | Negative | Critical |`Valid FK map` (holds mods) | 1. Locate Object holding >1 Folders.<br>2. Force Target delete via UI component manually/via API payload physically. | Request mechanically denied accurately via backend execution validation constraints explicitly blocking user operations ensuring disk-relationship purity remains undamaged. | S1 | AC-10.3.2, AC-10.3.3 |
| TC-10-09 | Pin logic mapping constraints | Positive | High |`Standard boolean parameter` | 1. Right click Unpinned UI block.<br>2. Pin element native interaction context.<br>3. Observe ObjectList. | Row natively ascends structure immediately sorting above general unmapped parameter counterparts effectively optimizing accessibility mapping contexts systematically. | S3 | AC-10.4.1, AC-10.4.2 |
| TC-10-10 | Ghost Pinning Reference Logic | Negative | Low |`Invalid logic ID` | 1. Delete Object externally via SQLite.<br>2. Target Pinning component via artificially removed payload structure parameters on UI. | Action softly reverts query logic maintaining stability and clearing list query constraints accurately avoiding runtime traps. | S3 | AC-10.4.3 |
| TC-10-11 | Pinned Stable Struct Scaling | Edge | High |`>50 Valid configuration tags mapped` | 1. Enable array tags mapping >50 elements.<br>2. Request reload physically.<br>3. Verify Sort sequence manually. | System strictly forces logical A-Z mapping algorithms exclusively against pinned boolean elements avoiding chaotic random array shuffles. | S3 | AC-10.4.4 |
| TC-10-12 | [Implied] Object name character limits | Implied | Med |`Payload length >128 chars` | 1. Open Object Creation.<br>2. Submit violently long String string parameter.<br>3. Verify database. | System trims data logic, or halts processing mechanically trapping excessive user bounds internally matching backend validation protocols. | S3 | N/A |

## D. Missing / Implied Test Areas

- **Schema category dropdown population**: When creating a new object, the dropdown for`categoryId` must be accurately synced to the _current_ active GameSchema, otherwise the user could assign a Genshin character into a ZZZ category organically via generic UI forms.

## E. Open Questions / Gaps

- "Name sanitization (trimmed, max 128 chars)" -> Trimming makes sense, but does the UI strictly prevent typing >128 chars via HTML limits, or does it only fail upon clicking submit?

## F. Automation Candidates

- **TC-10-08 (FK Preventative Locking)**: Native Rust testing logic asserting explicit execution blocks directly onto`delete_object` execution contexts evaluating Foreign Key relational integrity limits.
- **TC-10-01 (DB Insert -> Cache Flush)**: Cypress Testing frameworks mimicking UI input payload maps directly assessing DOM visual re-paints structuring logical mappings identical identical.

## G. Test Environment Setup

- **Preconditions**: SQLite DB mapped maintaining FK PRAGMA enforcements globally asserting physical constraints validating structural deletions.
- **Context Injection**: Seed >10 Objects containing exact mock dependencies triggering boundary limitations.

## H. Cross-Epic E2E Scenarios

- **E2E-10-01 (Safe Object Mutation Flow)**: User maps Mod to custom Object (Epic 07), executes Object Delete, triggering validation bounds that halt the deletion to maintain data (Epic 10) and emit an Error Toast (Epic 36).
