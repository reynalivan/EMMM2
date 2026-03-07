# Test Cases: Folder Listing & Classification (req-11)

## A. Requirement Summary

- **Feature Goal**: Backend command (`list_folders`) that reads a normalized sub-path, classifies each folder up to depth 5 recursively (`ModPackRoot`,`ContainerFolder`,`VariantContainer`,`InternalAssets`), enriches them with`info.json` + thumbnails, and handles prefix normalization.
- **User Stories**:
 - US-11.1: List Folder Contents
 - US-11.2: Normalization & Classification of`DISABLED` Prefix
 - US-11.3: Recursive Folder Classification
 - US-11.4: Metadata Enrichment
- **Success Criteria**:
 -`list_folders` ≤200ms for ≤500 immediate top-level items.
 - Classifier targets ≥ 95% entries based on`.ini` and texture structures.
 -`DISABLED` prefix stripping normalizes boolean variables 100% of the time.
 - Path traversal vulnerabilities entirely bounded by backend validation matching`mods_path`.
- **Main Risks**: Deeply nested Symlink loops locking thread execution natively (Rayon exhaustion). Excessive memory bounding during massive JSON evaluations triggering out-of-memory. Malformed regex stripping altering folder names destructively (e.g.`DISABLED` leaving total blank).
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-11-folder-listing.md`

- AC-11.1.1, AC-11.1.2 → TC-11-01
- AC-11.1.3 → TC-11-02
- AC-11.1.4 → TC-11-03
- AC-11.2.1, AC-11.2.2 → TC-11-04
- AC-11.2.3, AC-11.2.4 → TC-11-05
- AC-11.3.1, AC-11.3.2 → TC-11-06
- AC-11.3.3, AC-11.3.4 → TC-11-07
- AC-11.3.5 → TC-11-08
- AC-11.3.6 → TC-11-09
- AC-11.4.1, AC-11.4.2 → TC-11-10
- AC-11.4.3, AC-11.4.4 → TC-11-11

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ------------------------------------ | -------- | -------- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-11-01 | Basic List Folder performance | Positive | High | 500 immediate subfolders | 1. Populate`Mods/` directory with 500 flat generic folders.<br>2. Invoke`list_folders` with`sub_path=""` via Rust backend.<br>3. Measure total execution time.<br>4. Inspect the length of the returned array. | Native array mapping finishes ≤200ms. Returned array contains exactly 500 elements. No trailing slashes in mapped paths. | S2 | AC-11.1.1, AC-11.1.2 |
| TC-11-02 | Handling missing directory targets | Negative | High |`sub_path="NonExistent"` | 1. Execute`list_folders` pointing to`NonExistent` sub-path.<br>2. Evaluate IPC response.<br>3. Check Rust log for panic. | Command safely denies request mapping strictly returning`IO: NotFound` string error. App does NOT hard crash or panic. | S2 | AC-11.1.3 |
| TC-11-03 | Massive scale constraint loop | Edge | High | 10,000 nested folders | 1. Generate 10,000 folders nested 5 levels deep in test workspace.<br>2. Boot frontend grid mapping directory against massive structure.<br>3. Monitor Task Manager / System Activity. | Process uses`rayon` bounded by exact available thread count. Memory footprint peaks predictably without hitting OOM (out-of-memory). Evaluation finishes without freezing UI thread. | S2 | AC-11.1.4 |
| TC-11-04 | Standard boolean status mapped | Positive | High | 2 Folders:`KaeyaMod`,`DISABLED Amber` | 1. Populate workspace physically with`KaeyaMod` and`DISABLED Amber`.<br>2. Fetch Folder Array payload via`list_folders`.<br>3. Validate parsed output arrays. |`KaeyaMod` maps securely to parameter`is_enabled: true, name: "KaeyaMod"`.`DISABLED Amber` maps to`is_enabled: false, name: "Amber"`. | S2 | AC-11.2.1, AC-11.2.2 |
| TC-11-05 | Aggressive prefix parsing traps | Edge | Medium | Folder named`DISABLED DISABLED Skin` | 1. Create directory`DISABLED DISABLED Skin`.<br>2. Fetch Folder Array via`list_folders`.<br>3. Analyze normalizer array logic. | Double prefix strips. Output element maps parameter`is_enabled: false, name: "DISABLED Skin"`. It does NOT strip the second instance. | S3 | AC-11.2.3, AC-11.2.4 |
| TC-11-06 | Core Type Identification logic | Positive | High | Folder lacking`.ini` files | 1. Create`TextureDump` folder housing NO`.ini`, only generic`.txt` and`.png`.<br>2. Evaluate specific typing string return definition.<br>3. Create`WorkingMod` housing a valid`d3d11.ini`.<br>4. Re-evaluate. |`TextureDump` structurally evaluated as strict generic`ContainerFolder`.`WorkingMod` functionally mapped to specific`ModPackRoot`. | S2 | AC-11.3.1, AC-11.3.2 |
| TC-11-07 | Advanced internal categorization | Positive | High | Valid ini`[TextureOverride]` | 1. Create`SkinPack` with`info.json` and a`.ini` with a valid`[TextureOverrideKaeya]`.<br>2. Inside`SkinPack`, create`Body` and`Head` subdirectories.<br>3. Add`.txt` files to subdirectories to ensure they are parsed.<br>4. Validate internal referencing logic mappings string values internally. |`SkinPack` accurately maps type`ModPackRoot`.`Body` and`Head` structurally map to`ContainerFolder` by default since they lack internal`.ini` files. | S2 | AC-11.3.3, AC-11.3.4 |
| TC-11-08 | Infinite symlink loop trap | Negative | Critical | OS cyclic symlink | 1. Setup OS cyclic symlink internally inside the target Workspace path pointing back to root.<br>2. Traverse directory logic via`list_folders`.<br>3. Monitor backend for locking/hanging. | Mechanical logic aborts explicitly capping evaluation precisely at depth 5. Yields generic`ContainerFolder` for the symlink without causing core lockup/stack overflow. | S1 | AC-11.3.5 |
| TC-11-09 | Ambiguous Type sorting | Edge | Low | Base + Variant ini hybrid | 1. Create`HybridMod` folder containing a`.ini` file AND 5 valid subfolders each containing their own valid`.ini` files.<br>2. Trigger specific parser. | Safely maps strictly logical`ModPackRoot` maintaining precise structurally deterministic mappings avoiding ambiguous logical UI rendering behaviors visually. Priority rules declare`ModPackRoot` >`VariantContainer`. | S3 | AC-11.3.6 |
| TC-11-10 | Mapping Metadata parameters | Positive | High | Valid`info.json` payload | 1. Inject`info.json` with Author, Version, Link, and Category mapped into target folder.<br>2. Fetch folder array.<br>3. Check parsed mapped object elements internally. | Metadata components (`author`,`link` etc) securely parsed alongside absolute mapped Image URLs effectively validating standard UI structures. | S2 | AC-11.4.1, AC-11.4.2 |
| TC-11-11 | Malformed explicit metadata handling | Negative | Medium | Corrupted`info.json` | 1. Corrupt`info.json` mapping physically with invalid trailing commas and missing brackets.<br>2. Execute parser directory scanning.<br>3. Inspect payload return. | Malformed string parameters strictly return logic arrays mapping explicit`null` elements safely logging warning without blocking overarching file retrieval. | S3 | AC-11.4.3, AC-11.4.4 |
| TC-11-12 | Incremental Classification Cache | Positive | High | 500 folders, 1 changed | 1. Trigger full`list_folders` on 500 items. Record cache creation (keyed by path, mtime, size).<br>2. Change single byte in exactly 1 folder's`info.json`.<br>3. Re-trigger`list_folders`.<br>4. Measure execution time and logs. | Only the 1 stale entry is re-classified. The other 499 use cache. Total execution time ≤20ms (since ≤5% changed). Cache preserves processing cycles. | S1 | US-11.4 |
| TC-11-13 | False-Positive Protection | Negative | Medium |`.ini` with no valid sections | 1. Create`FakeMod` folder.<br>2. Add an`invalid.ini` file containing only generic text`[Settings] test=1` without any`TextureOverride*`,`ShaderOverride*`, or`Resource*` sections.<br>3. Trigger`list_folders`.<br>4. Check classification of`FakeMod`. |`FakeMod` is classified strictly as`ContainerFolder`, NOT`ModPackRoot`.`AC-11.3.6` enforces strict section requirement for modpack classification. | S2 | AC-11.3.6 |
| TC-11-14 | Detailed VariantContainer detection | Positive | High | 5 sibling sub-folders with`.ini` | 1. Create parent folder`MultiSkin`. It contains NO`.ini` itself.<br>2. Add 5 sub-folders (`Skin1` to`Skin5`).<br>3. Place a valid mod`.ini` in each of the 5 sub-folders.<br>4. Trigger`list_folders` on Workspace root.<br>5. Inspect`MultiSkin` model. |`MultiSkin` is classified specifically as`VariantContainer`. Its`variants[]` array contains exactly 5 elements mapping to the children.`is_navigable` is set. | S1 | AC-11.3.5 |
| TC-11-15 | referenced_subfolders extraction | Positive | High |`filename=./SubDir/...` in`.ini` | 1. Create`ComplexMod` with a valid`.ini` file.<br>2. Inside`.ini`, add a`[ResourceMyTex]` section with`filename=./InternalTexDB/texture.dds`.<br>3. Create`InternalTexDB` sub-folder alongside the`.ini`.<br>4. Trigger`list_folders`.<br>5. Inspect`ComplexMod` model.<br>6. Inspect`InternalTexDB` model. |`ComplexMod` is`ModPackRoot`. Its`referenced_subfolders[]` array contains`"InternalTexDB"`.`InternalTexDB` is specifically classified as`InternalAssets` (hiding it from variant navigation). | S2 | AC-11.3.4, AC-11.3.9 |
| TC-11-16 | Classification priority rule | Edge | High | Fits all 3 profile definitions | 1. Create`FrankensteinMod`.<br>2. Add a valid mod`.ini` (Matches`ModPackRoot`).<br>3. Add 5 sub-folders with valid`.ini`s (Matches`VariantContainer`).<br>4. Add random raw`.txt` files (Matches`ContainerFolder`).<br>5. Trigger`list_folders`. | Classification strictly resolves to`ModPackRoot`. The deterministic priority rule (ModPackRoot > VariantContainer > ContainerFolder) asserts dominance. | S2 | AC-11.3.8 |

## D. Missing / Implied Test Areas

- **Safe Mode Enforcement**: Requirement mentions Safe Mode in Section 3 Security, but no Acceptance Criteria covers it directly. Needs explicit test confirming`is_safe=false` objects are omitted from the array before returning.

## E. Open Questions / Gaps

- The classifier skips over`InternalAssets` natively so it doesn't return to the frontend. If a user manually renames an internal folder via external Windows Explorer to something that no longer matches the`[TextureOverride]` definition, it re-appears as a generic`ContainerFolder`. That behavior should be explicitly noted in docs for mod creators.

## F. Automation Candidates

- **TC-11-08 (Symlink depth bounding)**: Core Rust unit tests utilizing filesystem fixtures to construct cyclic links and asserting that the`list_folders` API forcibly terminates at depth 5 without exhausting stack memory.
- **TC-11-12 (Incremental Cache Performance)**: Rust benchmark test evaluating classification timing delta between cold-cache (1000 items) and hot-cache (1 item modified) measuring`std::time::Instant`.

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **FileSystem**: Write access to a dedicated`/test_workspace/Mods/` directory structure.
- **Dependencies**: Fake`gimi.json` resource pack mapping to ensure`info.json` deep linking logic clears.
- **Data Fixtures**:
 -`ValidMod/`:`.ini` with`[TextureOverride...]` present.
 -`VariantContainer/`: 5+ sub-folders parsing.
 -`OrphanedTex/`: No`.ini`, generic`.dds` inside.

## H. Cross-Epic E2E Scenarios

- **E2E-11-01 (Classification to UI Mapping)**: Execute`list_folders` on a complex nested filesystem (Epic 11). Validate that the frontend Folder Grid UI (Epic 12) strictly enforces Node-type navigation:`ModPackRoot` elements open Details Panels,`VariantContainer` objects explicitly spawn Variant Picker modals, and`InternalAssets` natively render null op.
