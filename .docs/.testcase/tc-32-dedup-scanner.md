# Test Cases: Smart Duplicate Scanner (Epic 32)

## A. Requirement Summary

- **Feature Goal**: Reclaim wasted disk space by scanning the active game`mods_path` using BLAKE3 hashing, finding identical binary assets (e.g., textures) across different mod folders, and allowing users to merge them via NTFS Hardlinks or delete them entirely via OS Trash.
- **User Roles**: Application User.
- **Acceptance Criteria**:
 - Paralleled multi-core BLAKE3 hashing`<15s` for 1000 files.
 - Phase 5: Multi-signal heavy hashing (1KB head/tail) for assets >5MB speeds up processing.
 - Phase 5: Hardlinks reduce size while maintaining valid paths. Deletion uses the OS Trash.
 - Phase 5: Handles locked handles without panic.
 - Phase 5: Report & Resolution DB caches scan results so users don't have to rescan immediately.
 - Phase 5: NTFS Hardlinks cross-drive fallback logic (gracefully handles`EXDEV` errors).
- **Success Criteria**: Deduplication never corrupts active mods. Hardlinks function transparently to the game engine.
- **Main Risks**: 3DMigoto failing to read hardlinked files (tested and proven safe). Cross-drive hardlink attempts crashing the app if not caught.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-32-dedup-scanner.md`

| Acceptance Criteria | Covered by TC IDs |
| :----------------------------------- | :---------------- |
| AC-32.1.1 (Multi-Signal Fast Hash) | TC-32-001 |
| AC-32.1.2 (Full BLAKE3 Verification) | TC-32-002 |
| AC-32.1.3 (Confidence Display) | TC-32-003 |
| AC-32.1.4 (Handle Locks Bypassed) | TC-32-004 |
| AC-32.1.5 (Ignore Defaults) | TC-32-005 |
| AC-32.2.1 (UI DB Sourced metrics) | TC-32-006 |
| AC-32.2.2 (Bulk Resolution Lock) | TC-32-007 |
| AC-32.2.3 (Prefix Normalization) | TC-32-008 |
| AC-32.3.1 (Trash Usage) | TC-32-009 |
| AC-32.3.2 (Cache Invalidation) | TC-32-010 |
| Phase 5: Multi-Signal Heavy Scan | TC-32-011 |
| Phase 5: Report & Res. DB | TC-32-012 |
| Phase 5: NTFS Hardlinks Cross-Drive | TC-32-013 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :--------------------------------- | :------- | :------- | :--------------- | :---------------------------------------------------------------------------------------------------- | :---------------------------------------------- | :------------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-32-001 | Base BLAKE3 Speed | Positive | High | S1 | 1000 identical 1MB files in`mods_path`. | 1000x 1MB`.dds` | 1. Initiate Duplicate Scan.<br>2. Monitor runtime logs. | Scan completes for all 1000 files in`<15s` utilizing multi-core processing. | AC-32.1.1 |
| TC-32-002 | Cryptographic BLAKE3 Verification | Positive | High | S1 | 4 files exist: 3 are exact copies. 1 has identical size/head/tail but 1 bit flipped in the middle. | 3 identical blobs, 1 tampered blob. | 1. Wait for scan verification phase.<br>2. Review output Payload. | The cryptographic pass catches the modified file. Groups only the 3 truly identical files together. | AC-32.1.2 |
| TC-32-003 | Match Confidence UI View | Positive | Medium | S2 | Matching set identical hashes, one has similar parent folder name (`Albedo_v1`,`Albedo_v2`). | Mod folders with similar names. | 1. Open Dedup Reporter UI. | Report groups identical hits and calculates structural similarity into a Confidence Percentage > 90%. | AC-32.1.3 |
| TC-32-004 | File Handle Locking Resilience | Edge | High | S1 | A target file is locked by another Windows process. | File`Texture.dds` held open. | 1. Start Scan.<br>2. Observe log stream. | Scanner logs`warn` explicitly for the locked file. Gracefully skips it without crashing the batch multi-core pipeline. | AC-32.1.4 |
| TC-32-005 | OS Hidden/System Pattern Whitelist | Negative | Low | S3 | Junk files like`desktop.ini` or`.DS_Store` are present in folders. | OS hidden files. | 1. Initiate Full Scan. | Junk files are hard-bypassed by memory structures natively and never appear in Duplicate lists. | AC-32.1.5 |
| TC-32-006 | DB SQLite Cached Views | Positive | High | S1 | User previously ran a scan. | Existing DB scan records. | 1. Open Dashboard.<br>2. Navigate to Scanner UI. | Table populates instantly (< 50ms) using DB persistent`duplicate_reports`. No drive re-hashing initiates on mount. | AC-32.2.1 |
| TC-32-007 | Bulk Action Operation Locking | Positive | High | S1 | 5 unique groups of duplicates exist in the UI. | Valid duplicated files. | 1. Select "Keep Original" for Group A.<br>2. Select "Replace" Group B.<br>3. Click "Apply All Changes". | Instructions run serially under`OperationLock`. Mutex isolates operations. Total OS bytes reclaimed. | AC-32.2.2 |
| TC-32-008 | Folder Prefix Normalization | Positive | Medium | S2 | Duplicates exist across active and disabled mods. | Folders named "Hu Tao" and "DISABLED Hu Tao". | 1. Evaluate scanner groupings. | The files group together inherently despite the "DISABLED " prefix due to binary signatures and underlying base naming. | AC-32.2.3 |
| TC-32-009 | OS Trash Path Targeting | Positive | High | S1 | User resolves a duplicate group via Deletion. | 3 grouped files set to "Delete". | 1. Click "Apply".<br>2. Manually check OS Trash via Windows Explorer. | All 3 files are removed from the working directory and moved directly into the OS proper Recycle Bin. | AC-32.3.1 |
| TC-32-010 | TanStack Expiration Refresh | Positive | Low | S3 | User just resolved 500MB of duplicates. | N/A | 1. Navigate back to the main Mods Manager Explorer grid. | Query`['mods', gameId]` triggers a re-fetch. Folder file sizes reflect the corrected smaller totals. | AC-32.3.2 |
| TC-32-011 | Multi-Signal Heavy Scan | Positive | High | S1 | User has 50 massive 50MB redundant video files. | 50MB files. | 1. Run Duplicate Scan. | The scanner uses the 1KB head/tail multi-signal phase first, dramatically filtering out non-matches before performing the full BLAKE3. | Phase 5 |
| TC-32-012 | Report & Res. DB Caching | Positive | Medium | S2 | User runs a scan, closes the app, and reopens it. | Mapped scan results. | 1. Run Scan.<br>2. Close App.<br>3. Open App, go to Dedup view. | The UI instantly loads the previous scan from SQLite`duplicate_reports` without re-hashing the disk. | Phase 5 |
| TC-32-013 | NTFS EXDEV Hardlinks Cross-Drive | Edge | High | S1 | The Game`mods_path` spans across simulated junction/symlinks pointing to a different physical drive. | Target resides on`D:\`, Link resides on`C:\`. | 1. Attempt "Hardlink" resolution across boundary. | Application catches`EXDEV` error. Aborts hardlink, warns user recommending 'Delete'. Original bits survive unharmed. | Phase 5 |

## D. Missing / Implied Test Areas

- **[Implied] Minimum Size Gate**: Validating the default`512KB` boundary ignores tiny`100 byte` duplicated text blobs saving CPU hashing cycles.

## E. Open Questions / Gaps

- No specific questions.

## F. Automation Candidates

- **TC-32-001 & TC-32-002**: Rust Integration mapping fake binary blob files to ensure Hash logic drops distinct differences.
- **TC-32-013**: Native OS level mock catching standard`fs::hardlink` failures on Windows architectures ensuring Rust`Result::Err` wraps around an EXDEV.
- **TC-32-009**: E2E verification mocking the`trash` crate to ensure paths align.

## G. Test Environment Setup

- **Test Data Directory**: Create multiple dummy copies of identical 10MB test binary`.dds` files across various folders (e.g.,`ModA/tex.dds`,`ModB/tex.dds`,`DISABLED ModC/tex.dds`).
- **Locked Handles**: Force a file lock on`ModD/locked.dds` using a background script or game executable loop (`File.OpenRead`).
- **Dual Drives**: If possible, mount a secondary volume or flash drive representing a disparate partition for the Cross-Drive EXDEV test.

## H. Cross-Epic E2E Scenarios

- **E2E-32-001 (Dedup Resolution to Folder Grid Reflection)**: The user navigates to the Settings Dashboard (Epic 33) and triggers a deep Dedup Scan. The backend calculates BLAKE3 hashes for 25GB of mods, caching the results into SQLite. The user opens the Scanner UI, selects 50 duplicate groups, and chooses "Hardlink". The backend processes these under`OperationLock`, replacing the redundant`.dds` files with NTFS hardlinks pointing to a single source of truth. The user navigates back to the main Folder Grid (Epic 12). The UI refetches the`mods` table and physical sizes. The user clicks the object, viewing the`info.json` metadata (Epic 40), and observes that the reported "folder size" has plummeted, confirming the space reclamation was successful and natively reflected across the entire app state.
