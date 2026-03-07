# Test Cases: Game Management (req-02)

## A. Requirement Summary

- **Feature Goal**: System for users to auto-detect and manually manage their game installations (Genshin, HSR, ZZZ, WuWa, Endfield) so mods are injected and processes spawned.
- **User Stories**:
 - US-02.1: Auto-Detect Games
 - US-02.2: Manual Game Addition
 - US-02.3: Remove Game
 - US-02.4: Launch Game
 - US-02.5: Active Game Switching
- **Success Criteria**:
 - Auto-detect ≤ 1s (5 drive roots max).
 - Manual add ≤ 300ms.
 - Mod Loader spawn ≤ 200ms.
 - Active game switch updates UI and queries ≤ 200ms.
 - No duplicate game records on spam clicks.
- **Main Risks**: Path validation bypasses resulting in unsafe file IO. Shell injections via launch args. Blocking UI thread with OS process spawning. Scanning loops from symlinks.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-02-game-management.md`

- AC-02.1.1, AC-02.1.2 → TC-02-01
- AC-02.1.3 → TC-02-02
- AC-02.1.4 → TC-02-03
- AC-02.1.5 → TC-02-04
- AC-02.2.1, AC-02.2.2 → TC-02-05
- AC-02.2.3 → TC-02-06
- AC-02.2.4 → TC-02-07
- AC-02.2.5 → TC-02-08
- AC-02.2.6 → TC-02-09
- AC-02.3.1 → TC-02-10
- AC-02.3.2 → TC-02-11
- AC-02.3.3 → TC-02-12
- AC-02.3.4 → TC-02-13
- AC-02.4.1 → TC-02-14
- AC-02.4.2 → TC-02-15
- AC-02.4.3 → TC-02-16
- AC-02.4.4, AC-02.4.5 → TC-02-17
- AC-02.4.6 → TC-02-18
- AC-02.5.1, AC-02.5.2 → TC-02-19
- AC-02.5.3 → TC-02-20
- AC-02.5.4 → TC-02-21

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | -------------------------------------------- | -------- | -------- | -------------------------- | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-02-01 | Auto-detect happy path | Positive | High | Folder with 3 game setups. | 1. Open "Add Game" modal.<br>2. Select "Auto Detect".<br>3. Point to parent folder.<br>4. Click "Scan". | Games parsed and assigned config within 1s. Valid paths for`mods_path`,`game_exe`,`loader_exe` stored. | S2 | AC-02.1.1, AC-02.1.2 |
| TC-02-02 | Auto-detect no valid games | Negative | Med | Empty/Random Folder | 1. Select "Auto Detect".<br>2. Point to`C:\\Windows`.<br>3. Click "Scan". | Empty list returned. No error dialogs shown. | S3 | AC-02.1.3 |
| TC-02-03 | Auto-detect permission denied | Negative | High | Protected generic Folder | 1. Select "Auto Detect".<br>2. Point to locked system folder.<br>3. Click "Scan". | UI displays clear validation error. App does not panic. | S2 | AC-02.1.4 |
| TC-02-04 | Auto-detect recursively nested symlinks | Edge | High | Deeply nested folder | 1. Create a symlink looping to its parent.<br>2. Point "Auto Detect" to it. | Scanner stops at depth 5 limit without infinite recursion/crash. | S2 | AC-02.1.5 |
| TC-02-05 | Manual Add happy path | Positive | High | Valid Game path & Type | 1. Open "Add Game" modal.<br>2. Select "Genshin Impact".<br>3. Input paths manually.<br>4. Click "Submit". | Game is validated, saved to DB in ≤ 300ms, set as`active_game_id`, router to`/dashboard`. | S1 | AC-02.2.1, AC-02.2.2 |
| TC-02-06 | Manual Add duplicate | Negative | Med | Exact existing path/type | 1. Input path mapping to existing DB row.<br>2. Click "Submit". |`DuplicateGame` error displayed in UI. No second row created. | S3 | AC-02.2.3 |
| TC-02-07 | Manual Add invalid path missing exe | Negative | High | Folder with no exes | 1. Input path containing no game executables.<br>2. Click "Submit". | Form shows inline`PathValidationError`. No DB record added. | S2 | AC-02.2.4 |
| TC-02-08 | Manual Add spoofed backend enum | Edge | High |`game_type="fake_game"` | 1. Intercept IPC and send invalid enum variant payload to backend. | Serde fails safely at boundary, returning typed error. Backend does not execute. | S2 | AC-02.2.5 |
| TC-02-09 | Manual Add rapid submit | Edge | Med | Valid Game info | 1. Fill valid info.<br>2. Rapidly spam exactly click "Submit" button 10+ times before modal closes. | Idempotent insertion occurs. Only 1 record created due to UNIQUE DB constraint. | S3 | AC-02.2.6 |
| TC-02-10 | Remove Game cascading | Positive | High | DB with full data | 1. Navigate to Settings > Games.<br>2. Click 'Remove' on target game.<br>3. Confirm in dialog. | Game removed. Mods and Objects related to the game are cascade-deleted immediately. | S1 | AC-02.3.1 |
| TC-02-11 | Remove Last Game | Negative | Med | 1 Game | 1. Navigate to Settings.<br>2. Remove the only configured game.<br>3. Confirm. |`active_game_id` is cleared. App immediately navigates to`/welcome`. | S2 | AC-02.3.2 |
| TC-02-12 | Remove non-existent Game ID | Negative | Low | Fake UUID | 1. Execute simulated IPC call`remove_game` with fake UUID payload. | Backend returns`NotFound`. State remains intact. | S4 | AC-02.3.3 |
| TC-02-13 | Remove Game while Scanning | Edge | High | 5GB of mods | 1. Trigger Mod Import Scan.<br>2. Navigate to Settings.<br>3. Remove game while scan runs. | Scanner is cancelled, file locks released, then database rows deleted. | S1 | AC-02.3.4 |
| TC-02-14 | Launch execution sequence | Positive | High | Valid paths | 1. Click 'Play' quickly. | Mod loader spawns (≤ 200ms) then game executable spawns as detached processes. | S1 | AC-02.4.1 |
| TC-02-15 | Launch auto-closes app | Positive | Med | Valid paths config | 1. Toggle`auto_close_on_launch = true`.<br>2. Click 'Play'. | EMMM2 terminates gracefully within 2s of successful launch. | S3 | AC-02.4.2 |
| TC-02-16 | Launch handles arguments | Positive | Med | Args:`-windowed -popup` | 1. Set explicit launch arguments.<br>2. Click 'Play'.<br>3. Check task manager process details. | The exact arguments are passed (verified via task manager / process explorer). | S3 | AC-02.4.3 |
| TC-02-17 | Launch files deleted manually | Negative | High | Stale DB reference | 1. Point config to exe.<br>2. Delete exe file.<br>3. Click 'Play'. | Process does not spawn.`IO: NotFound` error toast appears. App doesn't crash. | S2 | AC-02.4.4, AC-02.4.5 |
| TC-02-18 | Mod loader hangs UAC | Edge | Med | Process wait | 1. Point loader to exe requiring UAC prompt.<br>2. Click 'Play'.<br>3. Look at main EMMM2 UI. | UI thread remains active and unblocked because spawning happens async in Tokio. | S3 | AC-02.4.6 |
| TC-02-19 | Active game switch propagation | Positive | High | Game A, Game B | 1. Click top-bar active game dropdown.<br>2. Select Game B.<br>3. Check logs. | DB preference saved and`activeGameId` UI state updates in ≤ 200ms. File watcher restarts on new path and queries invalidate. | S2 | AC-02.5.1, AC-02.5.2 |
| TC-02-20 | Switch to invalid ID | Negative | Low | Null ID | 1. Send IPC`switch_game` with null UUID. | Frontend error. Game state unchanged. | S4 | AC-02.5.3 |
| TC-02-21 | Switch during Bulk Ops | Edge | High | Bulk disable | 1. Select 500 mods.<br>2. Click Toggle (triggers lock).<br>3. Attempt to switch active game in topbar simultaneously. | New watcher does not restart until the`OperationLock` is released. Target finishes. | S2 | AC-02.5.4 |
| TC-02-22 | [Implied] Launch path string contains spaces | Implied | Med | Path with spaces | 1. Add game located inside`C:\Program Files\Game\`.<br>2. Click 'Play'. | Process spawns accurately without CLI truncation/escaping issues. | S2 | N/A |

## D. Missing / Implied Test Areas

- **Game/Loader termination detection**: If the user has`auto_close_on_launch = false`, does the app track if the game dies to unlock any statuses, or is it purely detached 'fire and forget'? Requirement says detached, meaning we don't care after spawning. Verify the zombie process isn't left.
- **Spaces in Launch Arguments**: A launch argument argument like`--server "Global Server"` - are quotes passed literally or stripped by Rust`Command`?
- **Manual Add canonicalize paths on submission**: Does submitting`../../` inside`root_path` resolve to absolute before saving, avoiding DB reference corruption?

## E. Open Questions / Gaps

- "UI thread active while async spawner waits for UAC." -> If the user clicks Play multiple times while the async UAC is pending, does it spawn X loaders simultaneously?

## F. Automation Candidates

- **TC-02-05 (Manual Add Happy Path)**: Standard integration test. Add game to a mock folder context using Rust tests.
- **TC-02-09 (Duplicate submission idempotency)**: Unit/API test on`add_game()` confirming`ON CONFLICT` constraints.
- **TC-02-10 (Game cascading deletion)**: Backend database test verifying FK constraints using`sqlx`.
- **TC-02-19 (Active Game Switching Invalidates Query)**: React Testing Library functional test checking the cache reset hooks.

## G. Test Environment Setup

- **Preconditions**: Mock executables like`.bat` or`.ps1` acting as`loader.exe` that terminate quietly instead of booting heavy actual 30GB Games during E2E CI validations.
- **Database State**: Test cases like TC-02-01 expect an empty`games` table.
- **File System**: Ensure test paths mapped for games possess read access minimally.

## H. Cross-Epic E2E Scenarios

- **E2E-02-01 (Add Game -> Launch -> Auto-Close)**: Combine Manual Add Game (TC-02-05), verifying Dashboard render (Epic 33), toggling "Auto-Close" in Settings config, clicking "Play" (TC-02-14) executing process safely returning exit exactly confirming graceful end-to-end user session.
