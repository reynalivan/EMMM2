# Test Cases: Core Mod Operations — Toggle / Rename / Delete (Epic 13)

## A. Requirement Summary

- **Feature Goal**: Three critical user-facing operations on individual mods: (1) **Toggle** — flip enabled/disabled state via`DISABLED` prefix rename; (2) **Rename** — rename folder + update`info.json`; (3) **Delete** — move to OS Trash via`trash::delete`. All three use`OperationLock` +`WatcherSuppression` and return structured errors on failure with React Query optimistic rollback.
- **User Roles**: End User.
- **User Stories**: US-13.1 (Toggle), US-13.2 (Rename), US-13.3 (Delete/Trash).
- **Acceptance Criteria**:
 - AC-13.1.1: Toggle optimistic UI updates in ≤ 16ms; backend`fs::rename` completes ≤ 300ms.
 - AC-13.1.2: Toggle on slow HDD — UI switch animates immediately while IO runs in background.
 - AC-13.1.3: Folder locked by external process →`rename` fails; UI roll back + "Folder locked" toast.
 - AC-13.1.4: Rapid toggle spam (>3 clicks before rename completes) →`OperationLock` serializes; only final intended state applied; no double`DISABLED` prefix.
 - AC-13.2.1: Valid rename → folder renamed on disk (prefix preserved if disabled) +`info.json` name updated ≤ 500ms.
 - AC-13.2.2: Name with invalid chars (`\ / : * ? " < > |`) → frontend rejects before IPC call; shows inline error.
 - AC-13.2.3: Target path collision →`CollisionError` returned;`ConflictResolveDialog` opens; no partial rename.
 - AC-13.2.4: Path would exceed 260 chars →`PathTooLongError` before rename.
 - AC-13.3.1: Delete → moves to OS Recycle Bin (NOT`remove_dir_all`) ≤ 500ms.
 - AC-13.3.2: Delete of enabled mod →`enabled_count` decrements optimistically in objectlist ≤ 100ms.
 - AC-13.3.3: Trash unavailable → prompt "Permanently delete?" — hard delete only on explicit second confirm.
 - AC-13.3.4: Nested locked files →`PartialDeleteError`; original folder NOT moved to Trash (all-or-nothing).
- **Success Criteria**: 0 watcher re-fetches from app's own operations; 0 double-DISABLED folder names; no permanent deletion without explicit confirmation.
- **Main Risks**: OS lock on folder during game session blocks toggle; Trash full causes silent data loss if secondary prompt is skipped; concurrent toggle from two sources (UI + hotkey) before lock acquired.
---

## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :-------------------------------------- | :---------------- | :---------------------------------------------------------- |
| AC-13.1.1 (Optimistic toggle ≤ 16ms) | TC-13-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.1.2 (Slow HDD no UI lag) | TC-13-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.1.3 (Locked folder rollback) | TC-13-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.1.4 (Rapid toggle no corruption) | TC-13-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.2.1 (Valid rename + info.json) | TC-13-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.2.2 (Invalid chars rejected) | TC-13-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.2.3 (Collision dialog) | TC-13-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.2.4 (Path too long error) | TC-13-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.3.1 (Delete to Trash ≤ 500ms) | TC-13-009 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.3.2 (ObjectList count decrement) | TC-13-010 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.3.3 (Trash unavailable prompt) | TC-13-011 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| AC-13.3.4 (Partial lock all-or-nothing) | TC-13-012 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| Implied: Rename disabled mod prefix | TC-13-013 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |
| Implied: WatcherSuppression no re-fetch | TC-13-014 |`e:\Dev\EMMM2NEW\.docs\requirements\req-13-core-mod-ops.md` |

---

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :--------------------------------------------------- | :------- | :------- | :--------------- | :----------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :-------- |
| TC-13-001 | Optimistic UI toggle ≤ 16ms | Positive | High | S1 |`DISABLED KaeyaMod` visible in grid.`is_enabled = false`. Browser devtools Performance panel open. |`folder: "DISABLED KaeyaMod"` | 1. Open browser Performance recording.<br>2. Click toggle switch on`DISABLED KaeyaMod` card.<br>3. Stop recording.<br>4. Inspect the JS frame timestamp for the card's visual state change.<br>5. Wait ≤ 300ms.<br>6. Check disk:`KaeyaMod/` exists without prefix. | Toggle switch animates within ≤ 16ms of click. Card shows enabled state. Backend rename completes within ≤ 300ms. Folder on disk:`mods_path/Characters/Kaeya/KaeyaMod/` (no prefix). | AC-13.1.1 |
| TC-13-002 | No UI lag on slow storage | Positive | Medium | S3 | App connected to a network drive (simulated HDD with ≥ 100ms IO latency via mock).`DISABLED KaeyaMod` visible. | HDD/network path, IO latency = 200ms | 1. Configure mods_path to a throttled/slow mount.<br>2. Click toggle switch on`DISABLED KaeyaMod`.<br>3. Start stopwatch on click.<br>4. Observe toggle switch animation timing.<br>5. Wait for backend completion. | Toggle switch animates immediately within ≤ 16ms of click — no visible lag. The 200ms IO happens in background. Card remains in animated/pending state until backend confirms, then settles. User sees no stutter on the toggle control itself. | AC-13.1.2 |
| TC-13-003 | Locked folder rolls back toggle | Negative | High | S2 |`KaeyaMod` folder is locked by an external process.`is_enabled = true`. |`KaeyaMod/` locked externally | 1. Lock the folder: open`KaeyaMod/KaeyaMod.ini` in Notepad (keep open).<br>2. In app, click the toggle switch to disable`KaeyaMod`.<br>3. Wait for backend response.<br>4. Observe toggle switch state in UI.<br>5. Observe toast.<br>6. Check disk folder name. | Optimistic UI flips to disabled. Then backend`fs::rename` fails with OS error. Optimistic rollback fires: toggle switch flips back to enabled state. Toast shows: "Folder locked — cannot toggle." Folder still named`KaeyaMod/` on disk. DB is unchanged. | AC-13.1.3 |
| TC-13-004 | Rapid toggle spam — no double DISABLED prefix | Edge | High | S1 |`KaeyaMod` (enabled, no prefix).`OperationLock` running. | 5 rapid toggle clicks | 1. Click the toggle switch 5 times in rapid succession within 1 second.<br>2. Wait 500ms for all operations to settle.<br>3. Check disk folder name.<br>4. Check DB`is_enabled`. | Final folder name is either`KaeyaMod` (if final state = enabled) OR`DISABLED KaeyaMod` (if final state = disabled). Under NO circumstance is the name`DISABLED DISABLED KaeyaMod`.`OperationLock` serialized operations. | AC-13.1.4 |
| TC-13-005 | Valid rename updates disk + info.json | Positive | High | S1 |`KaeyaMod` (enabled) at`mods_path/Characters/Kaeya/KaeyaMod/`. Contains`info.json` with`"name": "KaeyaMod"`. | Old: "KaeyaMod", New: "KaeyaModV2" | 1. Right-click`KaeyaMod` card → "Rename".<br>2. Clear name field.<br>3. Type`KaeyaModV2`.<br>4. Press Enter or click Confirm.<br>5. Check disk.<br>6. Open`info.json` and read`name` field. | Folder on disk renamed to`KaeyaModV2/`.`info.json` shows`"name": "KaeyaModV2"`. Original`KaeyaMod/` no longer exists. All within ≤ 500ms. Card in grid shows updated name. | AC-13.2.1 |
| TC-13-006 | Invalid characters rejected before IPC | Negative | High | S2 | Rename dialog open for any mod. Frontend validation active. | New name:`Kaeya/Mod` (contains`/`) | 1. Open rename dialog.<br>2. Type`Kaeya/Mod` in the name field.<br>3. Observe any inline validation UI.<br>4. Try to submit.<br>5. Monitor browser Network tab for any IPC call. | Inline error message appears immediately: "Name contains invalid characters." Confirm button is disabled or submit is blocked. No IPC`invoke('rename_mod')` call fired. Folder on disk unchanged. | AC-13.2.2 |
| TC-13-007 | Collision triggers ConflictResolveDialog | Negative | High | S2 |`KaeyaMod` exists. ALSO:`KaeyaModV2` already exists at the same path (collision target). User tries to rename`KaeyaMod` to`KaeyaModV2`. | Target`KaeyaModV2` pre-exists | 1. Open rename dialog for`KaeyaMod`.<br>2. Type`KaeyaModV2` (which already exists).<br>3. Submit rename.<br>4. Observe modal/dialog opening.<br>5. Verify original`KaeyaMod/` state on disk. | Backend returns`CollisionError`.`ConflictResolveDialog` opens showing both`KaeyaMod` and`KaeyaModV2`. No partial rename occurs —`KaeyaMod/` is unchanged on disk. Original path still exists. | AC-13.2.3 |
| TC-13-008 | Path too long error before rename | Edge | Medium | S3 | Current folder path puts total near 240 chars. New name would push total path over 260 chars (Windows MAX_PATH). | Long path test | 1. Navigate to a mod in a deeply nested path (~240 chars total).<br>2. Open rename dialog.<br>3. Type a new name that adds ≥ 21 characters.<br>4. Submit rename.<br>5. Observe any error from backend. | Backend validates resulting path length BEFORE attempting`fs::rename`. Returns`PathTooLongError`. Toast shows: "Rename failed: resulting path would exceed 260 characters." No rename attempted. Folder unchanged. | AC-13.2.4 |
| TC-13-009 | Delete moves to Recycle Bin ≤ 500ms | Positive | High | S1 |`KaeyaMod` exists. OS Recycle Bin accessible. Confirmation dialog enabled in settings. |`folder: "KaeyaMod"` | 1. Right-click`KaeyaMod` card → "Delete".<br>2. Confirmation dialog appears → click "Delete".<br>3. Start stopwatch on confirmation click.<br>4. Wait for operation to complete.<br>5. Stop stopwatch.<br>6. Check disk.<br>7. Check Windows Recycle Bin. |`KaeyaMod/` moved to Windows Recycle Bin within ≤ 500ms. NOT permanently deleted. Folder visible and restorable from Recycle Bin. Card no longer visible in Explorer grid. | AC-13.3.1 |
| TC-13-010 | ObjectList enabled-count decrements ≤ 100ms | Positive | High | S2 |`KaeyaMod` is enabled (`is_enabled = true`). ObjectList shows "Kaeya: 3 enabled". | Enabled mod with count in objectlist | 1. Note objectlist count: "Kaeya: 3 enabled".<br>2. Click "Delete" on`KaeyaMod`.<br>3. Confirm.<br>4. Start timer immediately on confirmation.<br>5. Observe objectlist count within 100ms. | ObjectList "Kaeya" count drops to "2 enabled" within ≤ 100ms of confirmation, via optimistic UI update. Card disappears from grid. Full backend delete completes within ≤ 500ms. | AC-13.3.2 |
| TC-13-011 | Trash unavailable → explicit permanent delete prompt | Negative | High | S2 | OS Recycle Bin unavailable (full or restricted via policy). | Trash unavailable | 1. Block Trash access (mock`trash::delete` to return unavailable error).<br>2. Right-click`KaeyaMod` → Delete.<br>3. Click Confirm in first dialog.<br>4. Observe second confirmation dialog. | A secondary confirmation dialog appears: "Trash unavailable — permanently delete 'KaeyaMod'? This cannot be undone." Mod is NOT deleted until this second dialog is explicitly confirmed. If user cancels second dialog, mod remains on disk. | AC-13.3.3 |
| TC-13-012 | Partial lock → all-or-nothing, no partial trash | Negative | Medium | S2 |`KaeyaMod/` folder has nested file`KaeyaMod/textures/locked.dds` locked by OS. | Partially locked nested file | 1. Lock`KaeyaMod/textures/locked.dds` (e.g., open in image viewer).<br>2. Right-click`KaeyaMod` → Delete → Confirm.<br>3. Wait for error response.<br>4. Check disk:`KaeyaMod/` location.<br>5. Check Recycle Bin. | Backend returns`PartialDeleteError` listing`textures/locked.dds`. Original`KaeyaMod/` folder is NOT moved to Trash — remains fully intact on disk. Toast shows: "Delete failed: 'textures/locked.dds' is locked." Recycle Bin has no partial`KaeyaMod` remnant. | AC-13.3.4 |
| TC-13-013 | Rename of disabled mod preserves DISABLED prefix | Edge | High | S1 |`DISABLED KaeyaMod` exists (disabled). User types`NewKaeyaMod` (without prefix) in rename dialog. | Old: "DISABLED KaeyaMod", Input: "NewKaeyaMod" | 1. Open rename dialog for`DISABLED KaeyaMod`.<br>2. The dialog pre-fills "KaeyaMod" (stripping prefix for display).<br>3. Clear and type`NewKaeyaMod`.<br>4. Submit rename.<br>5. Check disk for new folder name. | Folder on disk renamed to`DISABLED NewKaeyaMod/` — prefix preserved by backend. The user's input`NewKaeyaMod` does NOT replace the prefix.`info.json``name` updated to`"NewKaeyaMod"` (no prefix in metadata). | Implied |
| TC-13-014 | Watcher prevents grid re-fetch on toggle | Edge | High | S2 | Active file watcher (Epic 28) running. Open React Query devtools. | Toggle any mod | 1. Open React Query devtools.<br>2. Toggle`KaeyaMod` (enable → disable).<br>3. Monitor React Query network request log during the 300ms backend window. | During and immediately after the toggle, zero unexpected`['folders', gameId]` query refetches caused by the file watcher. Only the intentional`invalidateQueries` from`onSettled` fires once. No duplicate grid renders caused by watcher loop. | Implied |

---

## D. Missing / Implied Test Areas

- **Rename + toggle interaction**: Rename`KaeyaMod` to`KaeyaV2` while a toggle is in progress under`OperationLock` — the rename should queue and execute after toggle completes.
- **info.json absent**: If`KaeyaMod/info.json` does not exist, rename should still rename the folder on disk without crashing —`info.json` update is skipped.
- **Delete from bulk selection**: This is tested in Epic 14 bulk operations; for Epic 13 scope, only single-item delete needs coverage.
- **Rename with identical name**: User opens rename dialog and submits the same name — should be a no-op (no rename, no error). Not stated in req but expected behavior.

---

## E. Open Questions / Gaps

- For`info.json`, does the rename command patch only the`name` field, or does it fully rewrite the JSON? (Req implies: patch only the`name` field — other keys preserved.)

---

## F. Automation Candidates

- **TC-13-004 (Rapid toggle)**: Rust integration test — dispatch 5`toggle_mod` calls concurrently, assert final folder name has exactly 0 or 1`DISABLED` prefixes.
- **TC-13-006 (Char rejection)**: Vitest + React Testing Library — render rename input, type each invalid char, assert`invoke` is never called and error message is visible.
- **TC-13-009 (Trash delete)**: Rust integration test — call`delete_mod`, assert`trash::delete` was called (spy), original path gone, no permanent deletion.
- **TC-13-014 (Watcher no re-fetch)**: Rust integration test — run toggle with mock watcher, assert watcher event count for the suppressed path = 0.

---

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build (`cargo tauri dev`). Game: Genshin Impact configured. DB State:`emmm2.db` with`KaeyaMod` indexed (`is_enabled` = true/false per TC). File Watcher (Epic 28) running locally.`OperationLock` released before each TC. OS Recycle Bin accessible.
- **Context Injection**:
 -`mods_path/Characters/Kaeya/KaeyaMod/` (enabled, with info.json)
 -`mods_path/Characters/Kaeya/DISABLED KaeyaMod/` (disabled, for toggle tests)
 -`mods_path/Characters/Kaeya/KaeyaModV2/` (pre-created for collision test)

## H. Cross-Epic E2E Scenarios

- **E2E-13-01 (Full Mod Lifecycle Execution)**: User imports Archive (Epic 23) which Auto-Organizes (Epic 38) and appears in Folder Grid (Epic 12). User executes Rename operation (Epic 13) triggering Metadata Sync (Epic 17). User triggers Toggle (Epic 13), updating ObjectList counters (Epic 06) and utilizing the file Watcher lock (Epic 28).
