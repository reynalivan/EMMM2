# Test Cases: Auto-Organizer (Epic 38)

## A. Requirement Summary

- **Feature Goal**: Move selected unorganized mod folders from a flat`Mods/` root into a structured`mods_path/{category}/{object_name}/{folder}` hierarchy using`fs::rename` under`OperationLock` +`WatcherSuppression`. Update`folder_path` in the DB atomically per move. Return a per-item`BulkResult`.
- **User Roles**: End User.
- **User Story**: As a user, I want to select unorganized mod folders and have the app move them to the correct hierarchy so that my filesystem stays clean.
- **Acceptance Criteria**:
 - AC-38.1.1: Moved to`mods_path/{category}/{object_name}/{folder_name}`, folder name preserved.
 - AC-38.1.2: DB`folder_path` updated atomically in the same transaction as the`fs::rename`.
 - AC-38.1.3: Toast shows "Organized N mods" + single`queryClient.invalidateQueries` call after full batch.
 - AC-38.1.4: If target already exists → skip, log as "DUPLICATE" in`BulkResult.errors`.
 - AC-38.1.5: If`object_id = NULL` (uncategorized) → skip, log as "NO_OBJECT".
 - AC-38.1.6: Enabled mods (no`DISABLED` prefix) are moved intact; enabled state is preserved.
- **Success Criteria**:
 - 100 folders organized in ≤ 10s on SSD.
 - Zero data loss on collision — original folder never moved to conflicting path.
 - 0 ghost DB entries after successful batch.
 - Exactly 1`invalidateQueries` call per organize batch (not N calls for N items).
- **Main Risks**: DB transaction rolls back after`fs::rename` succeeds → file at new path but DB points to old path (orphan); file already moved by watcher between lock acquisition and rename attempt.
---

## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :------------------------------------ | :---------------- | :------------------------------------------------------------ |
| AC-38.1.1 (Move to hierarchy) | TC-38-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| AC-38.1.2 (Atomic DB update) | TC-38-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| AC-38.1.3 (Toast + single invalidate) | TC-38-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| AC-38.1.4 (Duplicate skip) | TC-38-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| AC-38.1.5 (Uncategorized skip) | TC-38-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| AC-38.1.6 (Enabled mod move) | TC-38-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| Implied: 100-folder perf bound | TC-38-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |
| Implied: Path escape prevention | TC-38-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-38-auto-organizer.md` |

---

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :--------------------------------------- | :------- | :------- | :------------------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- | :----------------- |
| TC-38-001 | Standard move to hierarchy | Positive | High |`folder: "KaeqingMod_v2"`,`object_id` = Keqing,`gameId` = genshin | 1. Ensure App running, Genshin configured, folder`Mods/KaeqingMod_v2` exists with`object_id` pointing to`Characters > Keqing`,`mods_path/Characters/Keqing/` does NOT exist yet.<br>2. Open Explorer grid showing flat Mods root.<br>3. Right-click`KaeqingMod_v2` → context menu → "Auto-Organize".<br>4. Wait for operation to complete.<br>5. Open filesystem at`mods_path/Characters/Keqing/`.<br>6. Query DB:`SELECT folder_path FROM folders WHERE game_id = ? AND name = 'KaeqingMod_v2'`. | Folder exists at`mods_path/Characters/Keqing/KaeqingMod_v2/`. Folder name`KaeqingMod_v2` is unchanged. DB`folder_path` equals new path. Toast shows "Organized 1 mod". | S1 | AC-38.1.1 |
| TC-38-002 | DB update atomic with rename | Positive | High | Mocked DB UPDATE failure | 1. Setup as TC-38-001.<br>2. Trigger Auto-Organize on`KaeqingMod_v2`.<br>3. Inject DB failure via test hook after rename.<br>4. Check filesystem: new path existence.<br>5. Check DB:`folder_path` value.<br>6. Wait for next scan cycle (Epic 27 orphan repair). |`fs::rename` succeeds, file is at new path. DB UPDATE rolls back →`folder_path` still points to old path (orphan state). On next scan, Epic 27`repair_orphan_mods` reconciles. Toast shows an error. No data deleted. | S1 | AC-38.1.2 |
| TC-38-003 | Toast + single invalidation per batch | Positive | High | 3 folders with distinct objects | 1. Ensure 3 categorized mod folders selected:`AmborMod`,`KaeyaMod`,`LisaMod`, each with valid`object_id`.<br>2. Shift-select 3 folders in Explorer.<br>3. Right-click → "Auto-Organize".<br>4. Monitor React Query devtools for`invalidateQueries` calls.<br>5. Observe toast notification. | Toast reads: "Organized 3 mods". React Query`invalidateQueries(['folders', gameId])` fired exactly once after the full batch — NOT 3 times. Grid refreshes once. | S2 | AC-38.1.3 |
| TC-38-004 | Duplicate target skip | Negative | High | Both folders present | 1. Ensure folder`KaeqingMod_v2` exists in flat root AND`mods_path/Characters/Keqing/KaeqingMod_v2` already exists (collision).<br>2. Select`KaeqingMod_v2` from flat root.<br>3. Trigger Auto-Organize.<br>4. After completion, check: flat root folder existence.<br>5. Check`BulkResult.errors` (inspect from toast detail or logs). | Flat root`KaeqingMod_v2` is NOT moved — remains in place.`BulkResult.errors` contains`{ path: "KaeqingMod_v2", reason: "DUPLICATE" }`. Toast shows "Organized 0 mods, 1 skipped". No data overwritten. | S2 | AC-38.1.4 |
| TC-38-005 | Uncategorized mod skip (NULL object_id) | Negative | High |`object_id = NULL` in DB | 1. Ensure folder`SomeMysteryMod` exists with`object_id = NULL` in DB.<br>2. Select`SomeMysteryMod` in Explorer.<br>3. Trigger Auto-Organize.<br>4. Check: folder position on disk.<br>5. Check`BulkResult.errors`. |`SomeMysteryMod` stays in its current location.`BulkResult.errors` contains`{ path: "SomeMysteryMod", reason: "NO_OBJECT" }`. Toast: "Organized 0 mods, 1 skipped (no category)". | S2 | AC-38.1.5 |
| TC-38-006 | Move enabled mod preserves enabled state | Edge | High | Enabled folder | 1. Ensure`KaeqingMod_v2` exists (no`DISABLED` prefix),`is_enabled = true` in DB. Object_id = Keqing.<br>2. Note folder name:`KaeqingMod_v2` (no prefix).<br>3. Trigger Auto-Organize.<br>4. After completion inspect`mods_path/Characters/Keqing/`.<br>5. Check DB`is_enabled` for moved folder. | Folder exists at`mods_path/Characters/Keqing/KaeqingMod_v2` — still WITHOUT`DISABLED` prefix. DB`is_enabled = true` unchanged. 3DMigoto would still load this mod. | S2 | AC-38.1.6 |
| TC-38-007 | Performance: 100-folder batch | Edge | Medium | 100 × ~50MB folders | 1. Ensure 100 categorized folders in flat`Mods/` root, all having valid`object_id`. SSD storage.<br>2. Ctrl+A to select all 100 folders.<br>3. Right-click → Auto-Organize.<br>4. Record start time.<br>5. Wait for "Organized 100 mods" toast.<br>6. Record end time. | Full batch completes in ≤ 10 seconds. All 100 folders moved to correct hierarchy. All 100 DB rows updated. No UI freeze (progress indicator shown). | S3 | Implied (SC) |
| TC-38-008 | Path escape attempt blocked | Edge | High |`folder_path = "../../evil"` | 1. Ensure crafted folder name contains`../../../Windows/System32` or symlink pointing outside`mods_path`.<br>2. Inject crafted folder path via DB manipulation or test hook.<br>3. Trigger Auto-Organize via`invoke('auto_organize_mods', { game_id, folder_paths: [craftedPath] })`.<br>4. Check if rename was attempted outside`mods_path`. | Backend`canonicalize(target.parent()) + starts_with(mods_path)` check fails. Operation aborts with`CommandError::InvalidPath`. No rename to outside`mods_path` occurs. Log entry confirming rejection. | S1 | Implied (Security) |

---

## D. Missing / Implied Test Areas

- **Disabled Mod Move**: If`DISABLED KaeqingMod` is selected for organize, it should move to`mods_path/Characters/Keqing/DISABLED KaeqingMod` — the`DISABLED` prefix is part of the folder name and must be preserved in the target. (Not explicitly stated in req but follows consistency with toggle semantics.)
- **Sub-folder Files Preservation**: After auto-organize move, all sub-files (INI, textures, thumbnails) inside the mod folder must be intact — no partial move.
- **OperationLock Contention**: If another operation (e.g., toggle) holds`OperationLock` while Auto-Organize starts, what is the wait/fail behavior? (Implied: returns`CommandError::Locked` with toast.)

---

## E. Open Questions / Gaps

- On partial batch failure (e.g., 5 success, 2 skip, 1 error), does the toast consolidate all three counts into one message: "Organized 5, skipped 2, failed 1"?
- After an auto-organize, the old parent folder (e.g., flat`Mods/`) may now be empty. Does the app clean up empty parent directories, or leave them?

---

## F. Automation Candidates

- **TC-38-001 (Standard move)**: Rust integration test — create temp folder, call`auto_organize_mods`, assert new path + DB row.
- **TC-38-004 (Duplicate skip)**: Rust unit test on`BulkResult` — place conflicting folder, assert`errors[0].reason == "DUPLICATE"` and original untouched.
- **TC-38-005 (NULL object_id)**: Rust unit test — insert folder with`object_id = NULL`, call command, assert`errors[0].reason == "NO_OBJECT"`.
- **TC-38-003 (Single invalidate)**: Vitest / React Testing Library spy on`queryClient.invalidateQueries` — assert call count equals 1 after batch completion.

---

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Game**: Genshin Impact configured with valid`mods_path`
- **DB State**:`emmm2.db` with at least 3 mod folders indexed (`object_id` resolved by Deep Matcher)
- **Filesystem State**:
 - Create:`mods_path/KaeqingMod_v2/` with dummy`.ini` file
 - Create:`mods_path/AmborMod/`,`mods_path/KaeyaMod/`,`mods_path/LisaMod/`
 - Do NOT pre-create:`mods_path/Characters/Keqing/` (for positive tests)
- **File Watcher**: Running (to validate`WatcherSuppression` works correctly)
- **OperationLock**: Released (idle state) before each TC

## H. Cross-Epic E2E Scenarios

- **E2E-38-01 (Auto-Organize with Active Watcher Integration)**: Select 10 flat folders representing distinct Objects. Assert`WatcherSuppression` (Epic 28) explicitly holds around`fs::rename`. As they move, verify inherently that the File Watcher ignores precisely those 10 path modifications explicitly saving 10 duplicate intensive DB scanning cycles physically`<50ms`.`S2`.
- **E2E-38-02 (Mass Organize Conflict Resolution)**: Using identical source paths, simultaneously Auto-Organize multiple duplicate items actively into a folder physically causing Collision detection via Epic 39 integration throwing detailed errors without wiping original items.`S1`.
