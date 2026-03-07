# Test Cases: Sync Database (Epic 27)

## A. Requirement Summary

- **Feature Goal**: Commit authorized`ScoredCandidate` outputs into the SQLite database atomically, handle folder moves using BLAKE3 hashing, and purge deleted (orphaned) DB rows.
- **User Roles**: System (Automated transaction commit).
- **User Story**: As a system, I want to safely write approved scan mappings to the DB without duplicates. As a user, I want deleted directories to be purged from my DB automatically.
- **Acceptance Criteria**:
- Atomic upsert of DB`folders` mapping within transaction.
- BLAKE3 matching updates`folder_path` for moved directories rather than creating duplicate row.
- Interrupted runs roll back.
- Post-commit`repair_orphan_mods` removes phantom nodes.
- **Success Criteria**: 500 rows committed in <3s, zero duplicates produced, moved folders detected.
- **Main Risks**: Database locking during long transactions, unexpected duplicate constraints, UI cache invalidation mismatch.

## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :------------------------------- | :---------------- | :----------------------------------------------------------- |
| AC-27.1.1 (Atomic Upsert) | TC-27-001 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |
| AC-27.1.2 (BLAKE3 Move Logic) | TC-27-002 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |
| AC-27.1.3 (Transaction Rollback) | TC-27-003 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |
| AC-27.1.4 (Auto-Create Object) | TC-27-004 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |
| AC-27.2.1 (Orphan Delete) | TC-27-005 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |
| AC-27.2.2 (Toast Summary) | TC-27-006 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |
| AC-27.2.3 (Slow Disk Timeout) | TC-27-007 | `e:\Dev\EMMM2NEW\.docs\requirements\req-27-sync-database.md` |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :--------------------------- | :------- | :------- | :------------------------------------ | :-------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-27-001 | Commit approved candidates | Positive | High | 3`ScoredCandidate` records | 1. User approved 3 new mods in UI.<br>2. Click "Commit".<br>3. Execute`commit_scan`. | 3 new rows in`folders`, mapping`{folder_path -> object_id}` applied. | S1 | AC-27.1.1 |
| TC-27-002 | Committing a moved folder | Positive | High | Folder path changed, BLAKE3 identical | 1. Folder moved on disk.<br>2. Same INI content.<br>3. Execute`commit_scan`. | UPDATE`folder_path` for existing row instead of INSERT duplicate. | S2 | AC-27.1.2 |
| TC-27-003 | Transaction Rollback | Negative | High | Inject error after 2 inserts | 1. SQlite constraint failure injected during batch.<br>2. Execute`commit_scan`. | Transaction aborted, state reverts to pre-commit, no partial rows. | S1 | AC-27.1.3 |
| TC-27-004 | Target Object missing | Edge | Medium | Candidate mapped to new Object | 1. User mapped mod to "Dainsleif" (not in DB yet).<br>2. Execute`commit_scan`. | `Dainsleif` row implicitly created in`objects` before foreign key link. | S3 | AC-27.1.4 |
| TC-27-005 | Repair orphaned DB rows | Positive | High | Deleted folder on disk | 1. DB has row linking to`Old_Folder`.<br>2. Folder is deleted.<br>3. Run`repair_orphan_mods`. | DB row for`Old_Folder` removed, UI reflects removed item immediately. | S2 | AC-27.2.1 |
| TC-27-006 | Provide accurate stats toast | Positive | Medium | Mixed dataset | 1. Queue 10 added, 2 updated, 1 removed.<br>2. Finish`commit_scan`. | UI Toast lists exact`{added, updated, removed}` counts. | S3 | AC-27.2.2 |
| TC-27-007 | Repair orphan timeout | Negative | Low | Simulate 15s delay on disk`exists()` | 1. Very slow disk IO injected.<br>2. Run`repair_orphan_mods`. | Checks time out after 10s. Logs warning, retains unchecked rows. | S3 | AC-27.2.3 |
| TC-27-008 | [Implied] Idempotency | Positive | High | 2 Identical`ScoredCandidate` | 1. Queue identical records.<br>2. Execute`commit_scan` twice proactively. | Second run does nothing (`INSERT OR REPLACE` results in no change), no duplicates. | S1 | Implied |

## D. Missing / Implied Test Areas

- **[Implied] Concurrency Safety**: Ensure you cannot run`commit_scan` and an internal bulk toggle (`enable_only_this`) simultaneously leading to DB conflicts.

## E. Open Questions / Gaps

- No specific questions.

## F. Automation Candidates

- **TC-27-001, TC-27-002**: Pure database tests using`sqlx` inside a Rust test harness to prove INSERT/UPDATE conditional logic behaves as expected for BLAKE3 matching.
- **TC-27-003**: Integration test forcing a transaction rollback and verifying count queries remain unchanged.

## G. Test Environment Setup

- **Database Initial State**: Blank `folders` and `objects` tables. Provide an active mock connection string using SQLite in-memory `sqlite::memory:`.
- **File System Mocks**: Pre-generate structural test disk paths for missing Target logic simulating disk constraints.

## H. Cross-Epic E2E Scenarios

- **E2E-27-01 (Database Commit Pipeline)**: After the user verifies 100 ScoredCandidates from the Deep Matcher Modal (Epic 26), they click "Confirm & Sync". Epic 27's Database Synchronization handles the SQL transactions. If the process encounters a BLAKE3 conflict, it updates the absolute path instead of creating duplicates. The process also scrubs and repairs orphaned image rows.
