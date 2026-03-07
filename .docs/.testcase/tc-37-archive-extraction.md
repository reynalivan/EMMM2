# Test Cases: Archive Extraction Pipeline (Epic 37)

## A. Requirement Summary

- **Feature Goal**: Native archive extraction via`sevenz-rust` backend (ZIP, 7z, RAR) integrating pre-extraction`analyze_archive_cmd` to detect structure and encryption. Followed by`extract_archive_cmd` executing secure extraction into UUID-temp paths, applying smart-flattening, and utilizing`SuppressionGuards`.
- **User Roles**: Application User.
- **Acceptance Criteria**:
 -`analyze_archive_cmd` scans headers cleanly`<500ms`.
 - Prompts password safely if`is_encrypted=true` preventing blind extraction failures.
 - Automatically identifies redundant single-folder wrap hierarchies (e.g.`MyMod/MyMod/mesh.ib`) and flattens them.
 - Failures (bad passwords, bad CRC checksums) purge temporary un-extracted chunks seamlessly`<500ms`.
 - Extracted destinations evaluate natively against Epic 39 path collision guards securely prior to commit.
 - Phase 5:`sevenz-rust` backend explicitly tested for`RAR`,`7z`, and`ZIP` compatibility.
 - Phase 5: Progress Stream (`Tauri Event`) emits`[0-100%]` extraction progress to the UI.
 - Phase 5: Deep Folder Canonicalization (Handling`MAX_PATH` on Windows during extraction).
- **Success Criteria**: Users can drag and drop any supported archive format, optionally enter a password, and the mod installs flattened without leaving temporary garbage behind on failure.
- **Main Risks**: Extraction locking up the UI thread. Out of memory errors on multi-GB archives. Reaching the Windows 260-character path limit during extraction.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-37-archive-extraction.md`

| Acceptance Criteria | Covered by TC IDs |
| :----------------------------------- | :---------------- |
| AC-37.1.1 (Pre-Extraction Scan) | TC-37-001 |
| AC-37.1.2 (Encryption Gate) | TC-37-002 |
| AC-37.1.3 (Zero-Byte Archive) | TC-37-003 |
| AC-37.1.4 (Fragmented Block) | TC-37-004 |
| AC-37.2.1 (Smart Flattening 1-Level) | TC-37-005 |
| AC-37.2.2 (Multi-Root Preserved) | TC-37-006 |
| AC-37.2.3 (Decryption Failure Sweep) | TC-37-007 |
| AC-37.2.4 (Hex CRC Corruption Wipe) | TC-37-008 |
| AC-37.2.5 (External Collisions Gate) | TC-37-009 |
| Phase 5: Progress Stream`0-100%` | TC-37-010 |
| Phase 5: Format Compatibility | TC-37-011 |
| Phase 5: Windows MAX_PATH Handling | TC-37-012 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :----------------------------- | :------- | :------- | :--------------- | :---------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-37-001 | Fast Header Scan Metrics | Positive | High | S2 | App is running. UI is ready for drop. |`Clear_Big.zip` (1GB) | 1. Drop archive into UI. |`analyze_archive_cmd` parses structure emitting`{is_encrypted: false, root_folder_count: 1...}``<500ms` without decoding full internal blobs or hanging the UI Thread. | AC-37.1.1 |
| TC-37-002 | Encryption Password Interrupt | Edge | High | S2 | Archive is password protected. |`Locked.rar` | 1. Drop into UI. | Analysis accurately sets`is_encrypted: true`. UI explicitly mounts the`PasswordInputModal` strictly gating underlying extraction functions until a string is provided. | AC-37.1.2 |
| TC-37-003 | Blank Structural Format Shield | Negative | Medium | S3 | Archive is entirely empty. |`Empty.zip` | 1. Drop into UI. | Rust strictly detects`ini_count: 0 / file_count: 0`. Extraction is blocked pushing a Toast: "Archive contains no mod files". | AC-37.1.3 |
| TC-37-004 | Volume Parting Exigency Deny | Negative | Low | S3 | User drops a split archive part. |`Target.part1.rar` | 1. Execute drop. | System actively interprets separated multi-header formats, throwing a Toast "Multi-volume archives not supported" effectively blocking a broken extraction. | AC-37.1.4 |
| TC-37-005 | Single Layer Redundancy Fix | Positive | High | S2 | Archive has a redundant outer folder wrapper. |`Nested_Mod.7z` (`Skin/Skin/mesh.ib`) | 1. Initiate full un-packaging sequence. | System logic accurately identifies the redundant layer, extracts, and flattens structurally removing the exact wrapper root, mapping output into`mods/Skin/mesh.ib`. | AC-37.2.1 |
| TC-37-006 | Multi Component Root Hierarchy | Positive | Medium | S3 | Archive has multiple valid roots. | Archive containing`Root_A/`,`Root_B/`. | 1. Dump archive. | System respects structural definitions outputting both sibling directories into library path without breaking inter-mod relative definitions. | AC-37.2.2 |
| TC-37-007 | Bad Decryption Complete Sanity | Negative | High | S1 | Wrong password provided. |`Locked.rar` | 1. Feed explicit incorrect string over Password prompt.<br>2. Confirm. | Extraction execution inherently panics on key format. Wraps error sweeping the`app_temp_dir/uuid` natively into a blank state`<500ms` leaving no garbage. | AC-37.2.3 |
| TC-37-008 | CRC Decoding Failure Trap | Negative | High | S1 | Corrupted archive data. |`Corrupt.zip` | 1. Extract corrupted byte array. |`sevenz_rust` inherently throws mathematical checksum failure, rolling backwards I/O logic, full sweeping temp directories avoiding silently dropping broken apps. | AC-37.2.4 |
| TC-37-009 | Output Identifier Duplication | Edge | High | S2 | The extracted folder name explicitly matches a folder already existing in the destination root. | Extracted folder derives strictly output matched to folder pre-existing into directory bounds. | 1. Extract Mod onto identical string bounds. | Natively triggers explicit Epic 39 Conflict UI passing parameter explicitly suspending`SuppressionGuard`, waiting for user rewrite instruction. | AC-37.2.5 |
| TC-37-010 | Phase 5: Progress Stream UX | Positive | High | S2 | Large archive`> 500MB`. |`Large_Mod.7z` | 1. Drop Archive.<br>2. Watch UI. | Tauri Event channel`extraction_progress` emits payload`{ id: "uuid", progress: 45.5 }`. UI Progress Bar accurately fills up sequentially, never halting the main UI thread. | Phase 5 |
| TC-37-011 | Phase 5: Matrix Format Support | Positive | Critical | S1 | 3 identical archives packed in different formats. |`Test.zip`,`Test.7z`,`Test.rar` | 1. Drop all 3 sequentially. | The`sevenz-rust` backend flawlessly parses, analyzes, and extracts all three formats identically with zero data loss or unsupported format errors. | Phase 5 |
| TC-37-012 | Phase 5: Windows MAX_PATH | Edge | High | S1 | Archive contains deeply nested folders exceeding`260` characters. |`Deeply_Nested.zip` | 1. Drop archive into UI. | Rust uses`std::fs::canonicalize` or`\\?\*` path prefixing specifically on Windows targets to bypass the 260 limit, extracting without a silent filesystem exception crash. | Phase 5 |

## D. Missing / Implied Test Areas

## E. Open Questions / Gaps

- **Resource Limits**: Does`sevenz-rust` allocate entire archives into memory? Needs verification that memory mapping is used for large 10GB files to prevent OOM panics.
- **Rar5 vs Rar4**: Verify that the`sevenz-rust` crate officially supports the newer WinRAR`.rar5` dictionary format.

## F. Automation Candidates

- **TC-37-005, TC-37-006**: Pure backend integration inserting crafted standard`.zip` buffers executing specific nested arrays counting resulting physically derived output sequences.
- **TC-37-007, TC-37-008**: Feeding broken byte datasets verifying`Result::Err` outputs securely evaluating filesystem explicitly ensuring generic Temp path evaluates empty.

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Libraries**:`sevenz-rust` backend installed and functioning
- **Mock Archives**:
 -`Clear_Big.zip`: 1GB random bytes inside a single`ModDir` folder structure.
 -`Nested_Mod.7z`: Folder`Skin` contains Folder`Skin` containing`mesh.ib`.
 -`Locked.rar`: Protected via password "1234".
 -`Empty.zip`: A fully empty ZIP structural file containing exactly 0 bytes uncompressed data.
 -`Corrupt.zip`: Open a valid ZIP in HexEditor changing a byte inside compressed chunk forcefully failing its CRC.
 -`Deeply_Nested.zip`: A zip containing folder names 50 characters long nested 6 levels deep.

## H. Cross-Epic E2E Scenarios

- **E2E-37-001 (Safe Mode Archive Filtering)**: With Safe Mode strictly Active (Epic 30), a user extracts an archive`NSFW_Pack.zip` (Epic 37) containing known NSFW structural keyword folder names. The extraction succeeds mechanically to completion in the background, but the resulting physical folder is hidden accurately from the FolderGrid (Epic 11) respecting Safe Mode states.`S2`.
- **E2E-37-002 (Mass Import Duplicate Handling)**: A user drags and drops strictly 5 unique identically named archives simultaneously (Epic 23) queuing up 5 asynchronous`extract_archive_cmd` sequences (Epic 37). All 5 must extract avoiding single-instance global race conditions inside the`app_temp_dir` securely using UUID separation. All 5 then independently halt at the Epic 39 Boundary prompting the user for resolution exclusively avoiding database corruption.`S1`.
