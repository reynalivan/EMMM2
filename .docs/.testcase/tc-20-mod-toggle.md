# Test Cases: Mod Toggle Operations (Epic 20)

## A. Requirement Summary

- **Feature Goal**: Enable or disable a mod by adding/removing the`DISABLED` prefix on the physical folder, utilizing an optimistic UI,`OperationLock`, and`WatcherSuppression`. Collision detection acts as a safety against silently overwriting folders.
- **User Roles**: End User
- **User Story**:
 - US-20.1: Toggle Individual Mod
 - US-20.2: Handle Toggle Collisions
- **Success Criteria**:
 - Optimistic UI updates ≤ 16ms.
 - ObjectList`enabled_count` updates ≤ 50ms (optimistic mutation).
 - Physical`fs::rename` completes ≤ 300ms.
 - Pre-check collision detection prevents silent 100% data overwrite.
 - Rapid UI clicks (5 clicks in <1s) safely resolve without corrupting the end filesystem state or causing hanging UI.
- **Main Risks**: Interleaving multiple toggle calls breaking`OperationLock` or causing ghost folders like`DISABLED DISABLED MyMod`.`WatcherSuppression` failing and causing simultaneous React Query refetches mid-rename.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-20-mod-toggle.md`

- AC-20.1.1, AC-20.1.2 → TC-20-01
- AC-20.1.3 → TC-20-02
- AC-20.1.4 → TC-20-03
- AC-20.1.5 → TC-20-04
- AC-20.2.1, AC-20.2.2 → TC-20-05
- AC-20.2.3 → TC-20-06
- Implied Watcher Suppression → TC-20-07

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ----------------------------- | -------- | -------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-20-01 | Toggle Basic Behavior | Positive | High |`N/A` | 1. Select Mod`DISABLED MyMod` in grid.<br>2. Click toggle switch. | Mod is renamed to`MyMod`. Card reflects state in ≤ 16ms (optimistically) locally resolving physically. | S2 | AC-20.1.1, AC-20.1.2 |
| TC-20-02 | Badge Count Update | Positive | High |`N/A` | 1. Ensure ObjectList visible.<br>2. Click toggle on a Mod. | Total Enabled Count changes in ≤ 50ms without full refetch. | S3 | AC-20.1.3 |
| TC-20-03 | OperationLock Timeout | Negative | Med |`External rename ongoing` | 1. Force backend lock externally.<br>2. Attempt Toggle click in UI. | Lock Wait times out (3s). Returns "Operation in progress". Optimistic state rolls back. | S2 | AC-20.1.4 |
| TC-20-04 | Rapid Spamming Endurance | Positive | High |`Mod file` | 1. Spam click the toggle 10 times in 1 second. | Optimistic frontend debounce handles intent. Backend runs exactly one conclusive rename lock avoiding file corruption. | S1 | AC-20.1.5 |
| TC-20-05 | File Collision Flow | Negative | High |`Conflicting names` | 1. Create`MyMod` and`DISABLED MyMod` on disk.<br>2. Toggle`DISABLED MyMod` to enabled. | Throws`CommandError::Conflict`. UI triggers ConflictResolveDialog allowing Skip, Rename, Overwrite intercepting disaster. | S1 | AC-20.2.1, AC-20.2.2 |
| TC-20-06 | Overwrite While Trash Missing | Edge | High |`N/A` | 1. Same as TC-20-05, but Trash inaccessible.<br>2. Choose`Overwrite` natively from conflict dialog. | Overwrite canceled. "Cannot overwrite — Trash unavailable" toast presented. | S1 | AC-20.2.3 |
| TC-20-07 | Avoid File Watcher Recursions | Edge | High |`Mod file` | 1. Ensure Grid actively watching folder.<br>2. Toggle mod. |`WatcherSuppression` suppresses suppressing infinite state loops. | S2 | Implied |

## D. Missing / Implied Test Areas

- **Read-Only Permissions**: Attempting to toggle a folder with file permission blocks. (Implied:`CommandError::IoError` bubbles to toast).
- **Sub-folders Locking**: Attempting to toggle a parent folder while a file inside it is exclusively locked by a process (e.g., in Photoshop or Hex Editor). Windows blocks the rename operation; does the app bubble the IO error instead of partially renaming inner elements?

## E. Open Questions / Gaps

- The collision state triggers a ConflictResolveDialog. Is there a default choice if the Dialog times out, or is it strictly modal-blocking forever? (Implied: Explicit skip required by user).

## F. Automation Candidates

- **TC-20-01 (Optimistic Toggle state update)**: Essential feature functionality test (Vitest frontend mutation integration).
- **TC-20-04 (Rapid Toggle spam)**: Playwright test to simulate brutal high-latency user interactions.
- **TC-20-05 (Collision Rejection Lock check)**: Tauri E2E integration verification. Critical safety net confirming OS logic.

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Object Panel and File Watcher fully active.
- **Context Injection**:
 - Valid`DISABLED TargetMod` created via OS.
 - Conflicting`TargetMod` created directly beside it manually enforcing collision practically simulating standard worst-case physically.

## H. Cross-Epic E2E Scenarios

- **E2E-20-01 (Toggle Collision to UI Prompt)**: User navigates Explorer (Epic 15) and clicks Toggle on `DISABLED ModA` (Epic 20) while an active ModA already exists via an extracted archive (Epic 37). The backend rename intercepts the collision and returns a structured error. This triggers the Collision Resolver (Epic 39) to pause queues and avoid data destruction, prompting the user for resolution.
