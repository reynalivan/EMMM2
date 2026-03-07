# Test Cases: Mod Rename Operations (Epic 21)

## A. Requirement Summary

- **Feature Goal**: Allow users to safely rename mod folders, handling the`DISABLED` prefix state, avoiding collisions, updating`info.json` atomically, and performing a pre-delete/rename content audit.
- **User Roles**: End User
- **User Story**:
 - US-21.1: Rename Mod Securely
 - US-21.2: Pre-Delete Check (Content Audit)
- **Success Criteria**:
 - Rename completes ≤ 500ms on SSD.
 - Prefix logic handles enabled/disabled state 100% of the time.
 - Collisions rejected flawlessly via`OperationLock`.
 - Windows-invalid characters blocked prior to backend execute.
 - Rename-driven UI refresh uses purely optimistic + invalidate flows, not watcher events (via`WatcherSuppression`).
- **Main Risks**: Path Traversal vulnerabilities on rename input. Silent data deletion if moving over an existing folder. Desynchronization if the folder renames but`info.json` update crashes halfway. OS MAX_PATH exceptions crashing the backend thread.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-21-mod-rename.md`

- AC-21.1.1, AC-21.1.2 → TC-21-01
- AC-21.1.3 → TC-21-02
- AC-21.1.4 → TC-21-03
- AC-21.1.5 → TC-21-04
- AC-21.1.6 → TC-21-05
- AC-21.2.1, AC-21.2.2 → TC-21-06
- AC-21.2.3 → TC-21-07

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | --------------------- | -------- | -------- | ------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-21-01 | Rename Enabled Mod | Positive | High |`Target name: BetterName` | 1. Select Mod`MyMod` (enabled).<br>2. Trigger rename context action.<br>3. Enter`BetterName` and submit. | Folder becomes`BetterName` on disk.`info.json` name updates. UI optimistic update completes ≤ 500ms. | S1 | AC-21.1.1, AC-21.1.2 |
| TC-21-02 | Rename Disabled Mod | Positive | High |`Target name: NewName` | 1. Select Mod`DISABLED OldName`.<br>2. Trigger rename action.<br>3. Enter`NewName` and submit. | Folder becomes`DISABLED NewName`. Prefix is preserved. UI reflects. | S2 | AC-21.1.3 |
| TC-21-03 | Invalid OS Characters | Negative | High |`Target name: Bad:Name?` | 1. Select a Mod.<br>2. Trigger rename action.<br>3. Type`Bad:Name?`. | Frontend blocks submit button visually. Shows "Invalid characters in name". No IPC triggered securely preventing FS errors. | S2 | AC-21.1.4 |
| TC-21-04 | Path Collision | Negative | High |`Target name: Mod A` | 1. Ensure`Mod A` and`Mod B` exist.<br>2. Attempt to rename`Mod B` to`Mod A`. | Returns`CommandError::Conflict`. Triggers`ConflictResolveDialog` without overwriting. | S1 | AC-21.1.5 |
| TC-21-05 | MAX_PATH Overflow | Edge | Med |`128-char valid name` | 1. Select Mod inside deeply nested subfolder.<br>2. Rename to a length that exceeds 260 chars absolute path. | Backend returns`PathTooLongError`. Rename is aborted before FS operation avoiding severe data loss. | S2 | AC-21.1.6 |
| TC-21-06 | Pre-Delete Audit Flow | Positive | High |`Mod folder with 5 INIs` | 1. Initiate delete/rename on the folder heavily populated. |`pre_delete_check` returns stats ≤ 200ms. Dialog warns "This folder contains 5 INI files...". | S3 | AC-21.2.1, AC-21.2.2 |
| TC-21-07 | Extreme Folder Audit | Edge | Low |`Mod contains 50k files` | 1. Initiate delete/rename on a massive folder mapping. | Check times out after 2s without crashing. Continues flow presenting available stats. | S3 | AC-21.2.3 |

## D. Missing / Implied Test Areas

- **Empty Target String**: Submitting an empty string or whitespace-only string. (Implied: Form validation prevents it).
- **Missing info.json**: Renaming a folder that doesn't have an`info.json`. (Implied: It safely completes the direct folder rename operation without failing the IPC).

## E. Open Questions / Gaps

- "Renamed folder has no`DISABLED` prefix". What happens if the user manually types exactly`DISABLED MyNewMod` into the rename input for an enabled mod? Does the system strip their typed prefix, double it, or accept it as literal? (Requires explicit validation block or smart truncation).

## F. Automation Candidates

- **TC-21-01 / TC-21-02 (Core Rename State logic)**: Highly critical data destruction paths. Requires full end-to-end OS environment validation (Tauri FS mock testing or proper integration testing).
- **TC-21-04 (File Collision Lock)**: Extremely critical integrity check.

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. File Watcher fully active. Conflict resolver mounted.
- **Context Injection**:
 - Extremely deep nested folder structure constructed artificially to test`MAX_PATH`.
 - Mod folder seeded physically containing precisely 50,000 blank files validating`pre_delete_check` bounds.

## H. Cross-Epic E2E Scenarios

- **E2E-21-01 (Rename Integration Safety)**: User renames a Mod tracked across components (Epic 21). The frontend blocks illegal OS characters (Epic 36 Error Toast). A successful rename payload issues `WatcherSuppression` tokens to prevent recursive Folder Grid (Epic 15) re-renders. If the rename causes a backend collision, the Conflict Resolve Dialog (Epic 39) halts execution to protect existing files.
