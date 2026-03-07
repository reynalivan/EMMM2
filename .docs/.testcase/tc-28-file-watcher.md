# Test Cases: File Watcher (Epic 28)

## A. Requirement Summary

- **Feature Goal**: Run a live background filesystem monitor on`mods_path` that detects external creations, deletions, and renames, instantly pushing`fs-changed` events to the frontend while suppressing internal operations.
- **User Roles**: Application background service.
- **User Story**: As a user, I want the UI to reflect changes made via Windows Explorer instantly. As a system, I want to ignore changes I made myself via internal workflows to avoid grid re-rendering chaos.
- **Acceptance Criteria**:
 - Live external creations shown in ≤ 500ms.
 - Live external deletes disappear in ≤ 500ms.
 - Live external renames update grid within ≤ 500ms.
 - Internal bulk operations suppressed internally (0`fs-changed` emitted).
 - Panicked internal operations gracefully recover, removing suppression locks via RAII.
 - Watcher survives game switches.
- **Success Criteria**: 0 re-fetches fired from watcher during internal rename loops, ≤ 500ms OS delivery latency.
- **Main Risks**: Stale internal suppression guard locking paths indefinitely, duplicate events, high CPU usage during mass modifications.
## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :------------------------------- | :---------------- | :---------------------------------------------------------- |
| AC-28.1.1 (External Create) | TC-28-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| AC-28.1.2 (External Delete) | TC-28-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| AC-28.1.3 (External Rename) | TC-28-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| AC-28.1.4 (Root Deleted) | TC-28-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| AC-28.2.1 (Internal Suppression) | TC-28-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| AC-28.2.2 (Panicked Scope Drop) | TC-28-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| AC-28.2.3 (Colliding Events) | TC-28-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |
| N/A (Game Switch Watcher) | TC-28-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-28-file-watcher.md` |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :--------------------------------- | :------- | :------- | :----------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-28-001 | Real-time external file create | Positive | High | External folder creation tool | 1. Grid is rendering`mods_path`.<br>2. Create new mod folder`NewMod` in`mods_path/Characters/` via Windows Explorer. | The new folder pops up in`FolderGrid` inside ≤ 500ms. | S1 | AC-28.1.1 |
| TC-28-002 | Real-time external file delete | Positive | High | Mod folder`ToBeDeleted` | 1. Mod is visible in grid.<br>2. Delete`ToBeDeleted` physically via Explorer. | Row vanishes from`FolderGrid` inside ≤ 500ms physically. | S1 | AC-28.1.2 |
| TC-28-003 | Real-time external rename | Positive | High | Mod folder`OldName` | 1. Mod is visible in grid.<br>2. Rename to`NewName` physically via Explorer. | Grid replaces card`OldName` with`NewName` in ≤ 500ms. | S2 | AC-28.1.3 |
| TC-28-004 | Mods Path externally deleted | Negative | High | The active game's`mods_path` | 1. App is running.<br>2. Delete or move the root folder physically. | Watcher emits`fs-path-gone`; UI shows "Mods folder not found" alert. | S1 | AC-28.1.4 |
| TC-28-005 | Suppress internal operation | Positive | High | Bulk rename execution on 100 mods | 1. EMMM2 internal Bulk operation running.<br>2. Run an internal action that generates many`fs::rename`. | Watcher emits 0`fs-changed` events for those paths. | S1 | AC-28.2.1 |
| TC-28-006 | RAII Drop on panic | Negative | Medium | A mocked internal operation that panics | 1.`SuppressionGuard` active.<br>2. Start operation, trigger panic mid-way. | RAII naturally handles dropping lock,`suppressed_paths` are cleared immediately (< 100ms). | S2 | AC-28.2.2 |
| TC-28-007 | Colliding external/internal change | Edge | Low | External user changes exact same path as EMMM2 at same millisecond | 1. Suppressed internal renaming running.<br>2. User modifies exact same path via OS concurrently. |`fs-changed` is suppressed; Query cache updates after`staleTime` fallback (30s). | S3 | AC-28.2.3 |
| TC-28-008 | Watcher respawns on Game Switch | Positive | High | Genshin`mods_path` to HSR | 1. Switching from Genshin to HSR.<br>2. Trigger`set_active_game`. | Watcher binds to new HSR path in < 1s. Old path is unhooked. | S1 | implied |

## D. Missing / Implied Test Areas

- **[Implied] Debouncer Tolerance**: Ensure 100 rapid file creation events externally in <200ms trigger a single batch GUI update, not 100 render loops.
- **[Implied] Background Memory Usage**: Validate background`notify` service thread is dormant when there's no disk activity.

## E. Open Questions / Gaps

- No specific questions.

## F. Automation Candidates

- **TC-28-001, TC-28-002, TC-28-003**: A dedicated E2E script can spin up Tauri,`fs::write` to the local test folder, and assert React Query state updates via Playwright.
- **TC-28-005**: Mocking an internal rename loop and verifying the channel emits exactly 0 events over the wire.

## G. Test Environment Setup

- **Watcher Background Thread**: Background`notify` service initialized binding.
- **Folder Environment**: A physical dummy folder`mods_path` structurally identical simulating logical external file manipulation.

## H. Cross-Epic E2E Scenarios

- **E2E-28-01 (External Modification vs Grid Update)**: The user initiates Mod organization via Windows Explorer, adding 15 new archives (Epic 28). The File Watcher detects these structural updates and pushes debounced `fs-changed` signals to the Tauri Window. The frontend receives the event and explicitly invalidates the React Query cache for the affected paths, causing the Folder Grid to update without a full application refresh.
