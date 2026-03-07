# Test Cases: Folder Collision Resolution (Epic 39)

## A. Requirement Summary

- **Feature Goal**: When`fs::rename` fails with a name collision (e.g., both`Kaeya` and`DISABLED Kaeya` exist simultaneously), surface a`CommandError::PathCollision`; open a`ConflictResolveDialog` with side-by-side folder comparison data; let the user pick a resolution strategy (`KeepEnabled`,`KeepBoth`,`ReplaceWithIncoming`); auto-retry the original toggle after resolution.
- **User Roles**: End User.
- **User Story**: As a user, I want to see exactly what's inside both conflicting folders before deciding which to keep, so that I don't accidentally discard the better version.
- **Acceptance Criteria**:
 - AC-39.1.1:`ConflictResolveDialog` opens within ≤ 300ms of`CommandError::PathCollision`.
 - AC-39.1.2: Side-by-side panels show`{ file_count, total_size_bytes, thumbnail_path, ini_files }` per folder within ≤ 500ms.
 - AC-39.1.3: If a folder has no`.ini` files → show "No INI files found" (not an error).
 - AC-39.2.1: "Keep Enabled Version" →`DISABLED {name}` folder renamed to first free`DISABLED {name} (dup)` suffix;`{name}` folder untouched; toggle retried automatically.
 - AC-39.2.2: "Keep Both (Separate)" →`DISABLED {name}` renamed to`DISABLED {name} (copy)`; no toggle retried.
 - AC-39.2.3: "Replace With Incoming" → existing`{name}` moved to Trash;`DISABLED {name}` toggle completes to`{name}`.
 - AC-39.2.4: If suffixes`(dup)` through`(dup 10)` all exist → UUID suffix is used as final fallback.
 - AC-39.2.5: Closing dialog without choosing → original toggle cancelled; both folders untouched; no toast error.
- **Success Criteria**: ≤ 300ms dialog open; ≤ 500ms comparison data load; ≤ 1s for resolve + auto-retry. Zero data loss across all strategies.
- **Main Risks**: Auto-retry after resolution itself hitting another collision (nested collision). Trash unavailable for "Replace" strategy.`OperationLock` still held when resolution runs.
---

## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :----------------------------------------------- | :---------------- | :-------------------------------------------------------------- |
| AC-39.1.1 (Dialog open ≤ 300ms) | TC-39-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.1.2 (Side-by-side data) | TC-39-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.1.3 (No INI fallback display) | TC-39-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.2.1 (Keep Enabled strategy) | TC-39-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.2.2 (Keep Both strategy) | TC-39-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.2.3 (Replace With Incoming) | TC-39-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.2.4 (UUID fallback suffix) | TC-39-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| AC-39.2.5 (Dialog cancel) | TC-39-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |
| Implied: Replace strategy with Trash unavailable | TC-39-009 |`e:\Dev\EMMM2NEW\.docs\requirements\req-39-folder-collision.md` |

---

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :-------------------------------------------- | :------- | :------- | :---------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-39-001 | Dialog opens on PathCollision | Positive | High |`dir: "Kaeya"`,`dir: "DISABLED Kaeya"` | 1. Ensure folders`Kaeya/` AND`DISABLED Kaeya/` both exist in`mods_path/Characters/Kaeya/`.<br>2. In Explorer grid, locate`DISABLED Kaeya` card.<br>3. Click the toggle switch to enable it.<br>4. Observe: backend sends`CommandError::PathCollision`.<br>5. Start stopwatch from toggle click.<br>6. Wait for dialog to appear. |`ConflictResolveDialog` renders within ≤ 300ms. Dialog shows two labeled columns: "Current Active: Kaeya" and "Incoming: DISABLED Kaeya". | S1 | AC-39.1.1 |
| TC-39-002 | Side-by-side comparison data loads | Positive | High | 3-file folders with.ini | 1. Ensure collision dialog open from TC-39-001. Each folder contains 3 files including`KaeyaMod.ini`. One folder has a`preview.jpg`.<br>2. After dialog opens (from TC-39-001), start timer.<br>3. Wait for both panels to load.<br>4. Observe left panel values.<br>5. Observe right panel values. | Both panels populated within ≤ 500ms. Left/right each show: file count (exact integer), total size in human-readable format (e.g., "12.5 MB"), thumbnail preview image, list of`.ini` filenames (e.g., "KaeyaMod.ini"). | S2 | AC-39.1.2 |
| TC-39-003 | Folder with no INI files shows fallback | Edge | Medium | Texture-only folder | 1. Ensure collision dialog open. One of the conflicting folders contains NO`.ini` files (only textures).<br>2. Open collision dialog where one folder has 0`.ini` files.<br>3. Read the INI list section of that folder's panel in dialog. | That panel's INI list section displays exactly: "No INI files found" — rendered as text, not an error state and not an empty blank. Other panel still shows its INI list normally. | S3 | AC-39.1.3 |
| TC-39-004 | "Keep Enabled Version" strategy | Positive | High | Strategy: KeepEnabled | 1. Ensure collision dialog open. Both`Kaeya/` and`DISABLED Kaeya/` exist. Trash is accessible.<br>2. In`ConflictResolveDialog`, click "Keep Enabled Version" button.<br>3. Wait for resolution to complete.<br>4. Inspect filesystem: check`mods_path/Characters/Kaeya/` for all folders.<br>5. Verify toggle auto-retry completed. |`DISABLED Kaeya/` renamed to`DISABLED Kaeya (dup)/`.`Kaeya/` folder remains untouched. Toggle auto-retries:`DISABLED Kaeya` no longer exists; it would now complete to`Kaeya` (original). Grid refreshes. Total time ≤ 1s. Zero folders deleted. | S1 | AC-39.2.1 |
| TC-39-005 | "Keep Both (Separate)" strategy | Positive | High | Strategy: KeepBoth | 1. Ensure collision dialog open. Both`Kaeya/` and`DISABLED Kaeya/` exist.<br>2. In`ConflictResolveDialog`, click "Keep Both (Separate)" button.<br>3. Wait for resolution.<br>4. Inspect filesystem for all folder names in the Kaeya directory. |`DISABLED Kaeya/` renamed to`DISABLED Kaeya (copy)/`.`Kaeya/` stays untouched. NO toggle auto-retry fires. Both folders independently visible in Explorer grid. Zero files deleted. | S2 | AC-39.2.2 |
| TC-39-006 | "Replace With Incoming" strategy | Positive | High | Strategy: ReplaceWithIncoming, Trash accessible | 1. Ensure collision dialog open. Both`Kaeya/` and`DISABLED Kaeya/` exist. OS Trash is accessible.<br>2. In`ConflictResolveDialog`, click "Replace With Incoming" button.<br>3. Wait for resolution.<br>4. Inspect filesystem: check`Kaeya/` existence.<br>5. Check OS Recycle Bin for the displaced folder.<br>6. Verify toggle completed. | Existing`Kaeya/` folder moved to OS Recycle Bin (not permanently deleted). Toggle auto-retries:`DISABLED Kaeya/` renames to`Kaeya/`. Explorer grid shows updated`Kaeya`. Total time ≤ 1s. | S2 | AC-39.2.3 |
| TC-39-007 | UUID fallback when all dup suffixes taken | Edge | High | 10 pre-existing dup folders | 1. Ensure collision dialog open. Suffixes`DISABLED Kaeya (dup)` through`DISABLED Kaeya (dup 10)` all already exist on disk.<br>2. Pre-create`DISABLED Kaeya (dup)` through`DISABLED Kaeya (dup 10)` folders.<br>3. Trigger collision.<br>4. In dialog, choose "Keep Enabled Version".<br>5. After resolution, inspect all folder names in directory. | A new folder named`DISABLED Kaeya (dup) {uuid4}` is created (with a UUID suffix, e.g.,`DISABLED Kaeya (dup) a3f9...`). No iteration error. No infinite loop. All 11 existing dup folders remain untouched. | S2 | AC-39.2.4 |
| TC-39-008 | Dialog close cancels toggle | Negative | High | N/A | 1. Ensure collision dialog open. Both folders exist.<br>2. Open`ConflictResolveDialog` (from TC-39-001).<br>3. Click the ✕ close button WITHOUT selecting any strategy.<br>4. Inspect filesystem for both folders.<br>5. Verify UI state. | Both`Kaeya/` and`DISABLED Kaeya/` remain untouched on disk. No toast error appears. The toggle switch UI reverts to its original disabled state (optimistic rollback). DB state unchanged. | S2 | AC-39.2.5 |
| TC-39-009 | Replace strategy fails when Trash unavailable | Negative | High | Trash unavailable | 1. Ensure collision dialog open. OS Trash/Recycle Bin is unavailable (e.g.,`$Recycle.Bin` folder locked or full).<br>2. Mock or block Trash availability.<br>3. Open collision dialog.<br>4. Choose "Replace With Incoming".<br>5. Wait for response.<br>6. Inspect both folders on disk. |`trash::delete` fails. Toast shows: "Replace failed: Trash unavailable — cannot safely discard the existing folder." Both`Kaeya/` and`DISABLED Kaeya/` remain untouched on disk. No partial move occurred. | S2 | Implied |

---

## D. Missing / Implied Test Areas

- **Nested Collision on Auto-Retry**: After "Keep Enabled" renames the disabled folder, the auto-retry toggle could itself encounter yet another collision (if a third folder was created concurrently). The app should not loop indefinitely — after one auto-retry attempt, any second collision should abort and surface an error toast.
- **OperationLock during Dialog**: While the`ConflictResolveDialog` is open (user deciding), what keeps other operations from racing against the conflicting paths? (Implied: Lock is NOT held during dialog — user is reading comparison data; lock is only re-acquired during resolution execution.)
- **Large Folder Comparison Speed**: If each conflicting folder has 5,000 files,`get_collision_info` is bounded to`max_files_scanned = 100, max_depth = 3` — the dialog data loads fast even for huge folders; should show "+X more files" indicator.
- **Collision from Auto-Organizer (Epic 38)**: Collision can also originate from`auto_organize_mods` hitting a duplicate target. The same`ConflictResolveDialog` should work in that context too.

---

## E. Open Questions / Gaps

- For "Replace With Incoming", if`trash::delete` moves`Kaeya/` to Trash but then the toggle auto-retry fails, is there a rollback path? (Implied: manually restore from Trash Manager (Epic 22); no automatic rollback for trash moves.)
- Is there a "Cancel All" option in the dialog when this appears in a bulk operation context (e.g., 5 collisions in an auto-organize batch)?

---

## F. Automation Candidates

- **TC-39-004 (Keep Enabled)**: Rust integration test — create two conflicting folders, call`resolve_conflict(strategy: KeepEnabled)`, assert rename renaming succeeded and free suffix is correct.
- **TC-39-007 (UUID Fallback)**: Rust unit test on`find_free_suffix()` — pre-populate 11 conflicting names, assert returned suffix contains a UUID pattern.
- **TC-39-008 (Cancel)**: Playwright E2E — open dialog, press Escape, assert both folders unchanged and toggle state reverted in UI.

---

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Game**: Genshin Impact configured
- **DB State**:`emmm2.db` with both conflicting folders indexed
- **Filesystem State** (per test):
 - Create:`mods_path/Characters/Kaeya/Kaeya/` (enabled folder)
 - Create:`mods_path/Characters/Kaeya/DISABLED Kaeya/` (disabled folder)
 - For TC-39-007: also create`DISABLED Kaeya (dup)` through`DISABLED Kaeya (dup 10)`
- **Trash**: Accessible (Windows Recycle Bin enabled on system drive) unless testing TC-39-009
- **OperationLock**: Released before each TC
- **File Watcher**: Running (validates`WatcherSuppression` in resolution path)

## H. Cross-Epic E2E Scenarios

- **E2E-39-01 (Collections Collision Mitigation)**: Apply a Collection (Epic 31) that functionally requires 20 distinct folder`fs::rename` operations (Epic 20) rapidly. Inject a forced collision physically via the file system simultaneously on precisely the 10th mod. Verify the entire Collection execution effectively halts instantaneously without corrupting the overall atomic context explicitly triggering the Epic 39 Conflict Dialog visibly whilst explicitly preserving the state of all remaining unresolved items accurately without any background thread panic.`S1`.
- **E2E-39-02 (Mass Import Duplicate Handling Overlay)**: Drag and drop strictly 3 identically named, identically packaged Mod archives (Epic 23) sequentially triggering Epic 37 Extractor effectively resulting into 3 simultaneous parallel`CommandError::PathCollision` invocations resolving identical names.`S2`.
