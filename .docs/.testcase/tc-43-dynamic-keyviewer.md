# Test Cases: Dynamic KeyViewer Overlay (Epic 43)

## A. Requirement Summary

- **Feature Goal**: A two-layer offline + runtime system: (1) EMMM2 offline pipeline harvests hashes from enabled mod`.ini` files, scores against Resource Pack, selects sentinel hashes per object, generates`KeyViewer.ini` + per-object`{code_hash}.txt` keybind files atomically; (2) 3DMigoto runtime state machine uses sentinel hash hits with 5-step priority arbitration + anti-flipflop to display the correct character's keybind text overlay via`[Present]` PrintText.
- **User Roles**: Gamer, End User.
- **Acceptance Criteria** (Summary):
- Extracts hashes from enabled`.ini` files. Skips`.ini` files entirely lacking hash markers.
- Generates artifacts cleanly:`KeyViewer.ini` and text payloads.
- PrintText rendering bounds at 8KB or 60 lines max.
- Safe Mode actively filters out`is_safe=false` mod keybindings from text rendering.
- Phase 5: Sentinel Selection logic accurately implements the Object Scoring Formula (Base Value + Log Bonus) and enforces the explicit ambiguous Hash Collision Blacklist.
- Phase 5: Runtime State Machine arbitrates priority, TTL limits, and enforces a MIN_HOLD 0.20s anti-flipflop mechanic during combat/swapping.
- Phase 5: 6 explicit EMMM2 Event Triggers accurately command atomic`.ini` regeneration without blocking.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-43-dynamic-keyviewer.md`

| Acceptance Criteria | Covered by TC IDs |
| :-------------------------------------- | :---------------- |
| AC-43.1.1 (Hash extraction) | TC-43-001 |
| AC-43.1.5 (Duplicate merge) | TC-43-002 |
| AC-43.1.7 (Incremental scan) | TC-43-003 |
| AC-43.3.1 (Generated artifacts) | TC-43-004 |
| AC-43.3.4 (Safe Mode NSFW exclusion) | TC-43-005 |
| Phase 5: Object Scoring Formula | TC-43-006 |
| Phase 5: Ambiguous Hash Blacklist | TC-43-007 |
| Phase 5: PrintText Size Limit (8KB/60L) | TC-43-008 |
| Phase 5: Runtime Priority Arbitration | TC-43-009 |
| Phase 5: State Anti-Flipflop (0.20s) | TC-43-010 |
| Phase 5: 6 Regeneration Triggers | TC-43-011 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :------------------------------------ | :------- | :------- | :--------------- | :------------------------------------------------------------------------------------------------------ | :----------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-43-001 | Hash extraction from enabled mod INI | Positive | High | S1 | Mod`KaeyaMod.ini` is active, packed with`hash = AB12CD34` inside a TextureOverride. | Resource Pack loaded. | 1. Enable`KaeyaMod`.<br>2. Trigger feature update.<br>3. Inspect SQLite`mod_hash_index`. | `AB12CD34` physically maps to`Kaeya` in Db. System generates`[TextureOverrideKV_*]` entries. | AC-43.1.1 |
| TC-43-002 | Duplicate hash merge optimization | Positive | Medium | S2 | Two mods`KaeyaA` and`KaeyaB` share identical`AB12CD34` hashes. | 2 Mods, same Hash. | 1. Activate both sequentially.<br>2. Evaluate`KeyViewer.ini`. | `KeyViewer.ini` writes explicitly 1 set of Sentinels. Redundant copies logically deduped before artifact generation. | AC-43.1.5 |
| TC-43-003 | Incremental`.ini` caching | Edge | High | S2 | 200 Mods Enabled. Only 1.ini changes since last check. | `mtime` modification. | 1. Perform 1 full cycle.<br>2. Wait 2 seconds.<br>3. Edit a single mod's INI file slightly.<br>4. Trigger update Event. | Process skips the 199`.ini` files mapping. Harvest phase runs < 50ms total. | AC-43.1.7 |
| TC-43-004 | Output Artifacts Atomic Write | Positive | High | S1 | Normal mapping output to directories. | `EMM2/keybinds/active/` | 1. Force regeneration.<br>2. Monitor filesystem IO sequentially. | System writes rigidly to`.tmp` files. System invokes native OS`rename` commands overwriting old files, eliminating`3DMigoto` parsing conflicts. | AC-43.3.1 |
| TC-43-005 | Safe Mode Keybind Redaction | Edge | High | S1 | A NSFW mod explicitly provides`Tango = Remove Top` keybind text payload. | Safe Mode`ON`. | 1. Toggle Safe Mode ON.<br>2. System forces regeneration.<br>3. Read exact payload of`active/{code_hash}.txt`. | The explicit keybind strings defining NSFW commands are safely excluded from the generated `.txt` files without breaking the file structure. | AC-43.3.4 |
| TC-43-006 | Phase 5: Object Scoring Formula | Positive | High | S1 | A Character matches 3 different`known_hashes` from the Resource Pack. | Object:`Keqing`. Priority:`5`. | 1. Import Character.<br>2. Verify standard log metrics for sentinel ranking computation. | `score` evaluates to `priority (5) + log(hits)`. Ranked by highest priority. | Phase 5 |
| TC-43-007 | Phase 5: Ambiguous Hash Blacklist | Edge | High | S1 | A shader hash effectively maps to 5 unique characters simultaneously. | Blacklisted`hash`. | 1. EMMM2 accurately detects multi-character intersection logic.<br>2. It accurately maps the physical hash to`blacklisted_for_sentinel: true`.<br>3. Parses`KeyViewer.ini`. | The blacklisted hash contributes to detection but is excluded from valid `[TextureOverrideKV_*]` sentinels. | Phase 5 |
| TC-43-008 | Phase 5: PrintText Size Limit | Edge | High | S2 | Extremely large mod pack defining 120 keybind actions for`Gnar`. | Limit:`60 lines`. | 1. Export massive Mod.<br>2. Generate payload.<br>3. Parse`{hash}.txt` filesize. | The text payload automatically truncates at Line 60, appended with a `...` ellipsis terminator to avoid 3DMigoto layout crashes. | Phase 5 |
| TC-43-009 | Phase 5: Runtime Priority Arbitration | Positive | Critical | S1 | 3DMigoto State Machine logic. Two intersecting Object Sentinels hit on the identical frame. | Frame Collision. | 1. Execute Game.<br>2. Simulate rendering both Character and general Weapon.<br>3. Observe textual Banner. | In-game logic defaults to the `code_hash` mapping with the highest Priority. | Phase 5 |
| TC-43-010 | Phase 5: State Anti-Flipflop (0.20s) | Positive | High | S2 | Mid-combat weapon swapping. Character sentinel drops momentarily but is expected to return. | Interval:`0.1s`. | 1. Execute Game.<br>2. Hit Character Sentinel.<br>3. Lose Character Sentinel for 3 frames (< 0.20s).<br>4. Validate Text Render Logic. | The textual KeyViewer overlay remains stable, ignoring micro-interruptions under the 0.20s threshold. | Phase 5 |
| TC-43-011 | Phase 5: 6 Regeneration Triggers | Positive | High | S1 | The exact 6 defined explicit Events that must queue structural Regenerations. | Global Events. | 1. Mod Toggle.<br>2. Profile Swap.<br>3. File Watcher Event.<br>4. Custom Keybind modification.<br>5. Resource Pack App Update.<br>6. Safe Mode Toggle. | All 6 defined Events independently execute the structural `.tmp` pipeline without duplication. | Phase 5 |

## D. Missing / Implied Test Areas

- **Elevated Contexts**: Does the`enigo` keypress synthesize strokes into the Game window if the Game is running strictly As Administrator but EMMM2 is running natively as Standard User? (Likely blocked by Windows UIPI - needs fallback documentation).

## E. Open Questions / Gaps

- When paging is enabled (optional), what key clears the page index? The req says "page state resets on object change" — is there also a manual reset key?

## F. Automation Candidates

- **TC-43-001 (Hash extraction)**: Rust unit test — call`extract_hashes_from_ini_text` on a test INI string, assert correct hash set.
- **TC-43-007 (Blacklist)**: Rust unit test — create 3 Resource Pack entries sharing a hash, run scoring, assert`blacklisted_for_sentinel = true`.
- **TC-43-003**: Integration script mapping exactly 5,000 blank mock files sequentially, executing cache checks and benchmarking execution time.

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Game**: Genshin Impact configured with Resource Pack (`gimi.json`) loaded
- **KeyViewer Output Paths**: -`Mods/.EMMM2_System/KeyViewer.ini` -`EMM2/keybinds/active/{code_hash}.txt` -`EMM2/keybinds/active/_fallback.txt`
- **3DMigoto**: Running
- **KeyViewer Feature Toggle**: ON in EMMM2 settings

## H. Cross-Epic E2E Scenarios

- **E2E-43-001 (Collections -> KeyViewer Workflow)**: Execute Epic 42 hotkey`F6` to swap to a new Collection preset. Validate that the Epic 43 KeyViewer pipeline automatically regenerates the`KeyViewer.ini` and text artifacts to precisely match the newly enabled characters within ≤ 1s without hanging the game or requiring manual refreshes.`S1`.
- **E2E-43-002 (Mass Import + Atomic Writing)**: Drag and drop 5 character mods sequentially (Epic 23 mass import), triggering 5 rapid file watcher events in succession. Validate that Epic 43 accurately schedules the generation task (using debouncing or atomic`.tmp` renames) so that the final`KeyViewer.ini` structurally contains all 5 character sentinels without a corrupted or truncated INI file.`S1`.
