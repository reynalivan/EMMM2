# Test Cases: Object Schema & Master Database (req-09)

## A. Requirement Summary

- **Feature Goal**: Provide a dynamic bundled`schema.json` format mapping per-game object hierarchies/taxonomies and implementing a Master Database lookup system mapping raw directory string titles directly towards accurate canonical character targets automatically.
- **User Stories**:
 - US-09.1: Game Schema Enforcement
 - US-09.2: Master Database Name Resolution
- **Success Criteria**:
 - Assets validate structurally during backend execution boot ≤100ms.
 - Categories build appropriately depending on runtime switch context.
 - Fast Memory lookup mapped array resolutions handle ambiguous naming (≤5ms/target).
 - Absolute graceful degradation against entirely missing asset mappings defaulting natively into local folder name proxies 0 crushes.
- **Main Risks**: Missing bundle files during compilation causing hard core startup panics (`tauri::include_str!`), Unidentified alias collisions auto-moving structurally dangerous mod structures incorrectly into overlapping hero targets.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-09-object-schema.md`

- AC-09.1.1 → TC-09-01
- AC-09.1.2 → TC-09-02
- AC-09.1.3 → TC-09-03
- AC-09.1.4 → TC-09-04
- AC-09.2.1, AC-09.2.2 → TC-09-05
- AC-09.2.3 → TC-09-06
- AC-09.2.4 → TC-09-07

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | -------------------------------- | -------- | -------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-09-01 | Dynamic Schema Hydration | Positive | High |`GameType switches` | 1. Open App.<br>2. Swap Context from Schema A (Genshin) to Schema B (ZZZ) via Topbar.<br>3. Inspect ObjectList. | UI headers, taxonomy properties distinctively adapt matching JSON`category_id` references effectively avoiding hardcoded labels. | S2 | AC-09.1.1 |
| TC-09-02 | Missing JSON Fallback | Positive | High |`Missing Schema JSON` | 1. Purge targeted JSON schema payload from binaries/disk.<br>2. Boot Application. | Does not white-screen crash. Renders simplified "Mods" singular hierarchical mapping securely utilizing fallback automatically. | S2 | AC-09.1.2 |
| TC-09-03 | Hard JSON Corruption trap | Negative | Med |`Fatal structural typing errors internally` | 1. Corrupt schema JSON syntax manually.<br>2. Boot Application. Backend validation phase initialized. | App halts startup pipeline emitting distinctly clear textual trace error blocks specifically outlining invalid boundary targets vs silent fail corruption. | S1 | AC-09.1.3 |
| TC-09-04 | Categorical Drift Mapping | Edge | Med |`Outdated DB object` | 1. Provide old DB object instance missing category enum ID in modern context via manual SQLite edit.<br>2. Load App. | Safely isolates elements exclusively under dynamic "Uncategorized" array parameters preventing mapping exceptions or invisibility. | S3 | AC-09.1.4 |
| TC-09-05 | Dictionary Match performance | Positive | High |`Raw query ("RaidenShogun_Mod")` | 1. Trigger Mod Import Scan.<br>2. Run parser logic checking aliases against Master HashMap.<br>3. Profile execution. | String structurally resolved mapping specifically towards clean`"Raiden Shogun"`. Computes via indexed structs (≤5ms). Assessed. | S2 | AC-09.2.1, AC-09.2.2 |
| TC-09-06 | Null Database resolution | Negative | Low |`Missing master_db.json` | 1. Erase/hide`master_db.json`.<br>2. Initiate parsing target sequence. | Fails accurately reverting identities matching baseline physical folder parameters identically without crashing application sequences. | S3 | AC-09.2.3 |
| TC-09-07 | Aliasing Conflict traps | Edge | High |`"Traveler"` string collision | 1. Provide physical folder matching "Traveler".<br>2. Parser executes upon multi-targeted contextual word sequence ("Aether", "Lumine"). | Suspends mechanical direct assignment automatically avoiding heuristic guesswork mapping elements towards generic user interaction disambiguation workflow flag. | S2 | AC-09.2.4 |
| TC-09-08 | [Implied] Stopword parsing strip | Implied | Med |`Folder: "Hu Tao v2 By Modder [Busty]"` | 1. Input noisy folder path into scan engine.<br>2. Run matcher algorithm against schema configured stopwords (`"v2"`,`"By"`, etc). | Output appropriately strips configured noise tokens natively before executing dictionary checking. | S3 | N/A |

## D. Missing / Implied Test Areas

- **Schema Refreshing on Runtime Updates**: If an over-the-air update downloads a new bundled schema JSON via Epic 34, does the`Arc<RwLock>` re-read from disk, or does it require an explicit background application restart?
- **Alias case insensitivity**: The dictionary must map`rAiDeN` to`Raiden Shogun`. Is canonical matching strictly case-insensitive inside Rust?

## E. Open Questions / Gaps

- "Schema loads embedded at compile time" -`include_str!` bakes it into the executable binary. This means it physically _cannot_ be "missing or corrupt" (AC-09.1.2 and AC-09.1.3) at runtime for end users because rustc would refuse to compile. These ACs during development or if the architecture actually reads from an external app_data dir.

## F. Automation Candidates

- **TC-09-05 (Master DB Matching Accuracy)**: Rust Unit Test. Directly testing`resolve_name` against a fixture dataset of 200 distinct naming configurations proving successful mapping parameters are greater than 90%.

## G. Test Environment Setup

- **Preconditions**: Rust test execution frameworks populated natively importing active raw JSON Master lists containing real-world `schema.json` data to validate correct serialization logic.
- **Data Fixtures**: 500+ mock folder paths holding dirty stop words, underscores, bad unicode to stress the Master ID mapper.

## H. Cross-Epic E2E Scenarios

- **E2E-09-01 (End-to-End Object Sync)**: Scan Engine (Epic 25) retrieves raw dirty path strings natively feeding Deep Matcher (Epic 26) applying Schema validation (TC-09-05) injecting classified rows, triggering Database Sync (Epic 27), which updates the ObjectList (Epic 06) to reflect the newly classified data.
