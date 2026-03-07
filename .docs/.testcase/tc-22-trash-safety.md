# Test Cases: Trash Safety System (Epic 22)

## A. Requirement Summary

- **Feature Goal**: Execute an unbreakable "zero data loss" mandate for folder deletions using OS Trash (`trash` crate) with a custom`.trash/` fallback. Provide a Trash Manager feature to restore or securely flush soft-deleted items.
- **User Roles**: End User
- **User Story**:
 - US-22.1: Soft Delete (Move to Trash)
 - US-22.2: Trash Manager (In-App Recovery)
- **Success Criteria**:
 -`trash::delete()` executes in ≤ 500ms.
 - Relational Database references cascade with item drop in a single atomic transaction.
 - Custom fallback`{app_data_dir}/.trash/{uuid}/` used exclusively as a failsafe during cross-drive copy restriction or OS trash malfunction.
 - Hard deletion runs only upon explicit second confirmation inside "Empty Trash" manager context. Restore executes within ≤ 500ms.
- **Main Risks**: Desynchronized DB states where folder vanishes but UI/DB persists ghosts, locking operations indefinitely because of open file handlers in 3DMigoto preventing soft moves, and disk capacity filling unexpectedly with app-managed`.trash/` files.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-22-trash-safety.md`

- AC-22.1.1, AC-22.1.2 → TC-22-01
- AC-22.1.3 → TC-22-02
- AC-22.1.4 → TC-22-03
- AC-22.2.1 → TC-22-04
- AC-22.2.2 → TC-22-05
- AC-22.2.3 → TC-22-06
- AC-22.2.4 → TC-22-07
- AC-22.2.5 → TC-22-08

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | --------------------------- | -------- | -------- | ----------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-22-01 | Standard Soft Delete | Positive | High |`Folder < 1GB` | 1. Select Mod folder mapped to active object.<br>2. Click Delete.<br>3. Confirm default dialog. | Folder resolves to OS Recycle Bin physically.`folders` DB record wiped atomically. ObjectList counts update accurately ≤ 500ms. | S1 | AC-22.1.1, AC-22.1.2 |
| TC-22-02 | OS Trash Failure Fallback | Negative | High |`Folder` | 1. Force mock`trash::delete()` error (mock cross-drive).<br>2. Initiate delete. | App intercepts error relocates folder mechanically to`{app_data_dir}/.trash/uuid/`. DB tracks. | S1 | AC-22.1.3 |
| TC-22-03 | Active Lock Rejection | Edge | High |`Folder` | 1. Lock Mod file externally (simulate Game Running).<br>2. Initiate delete. | System returns OS lock error. Toast warns "Cannot delete - game may be running". Folder and DB stay physically intact. | S2 | AC-22.1.4 |
| TC-22-04 | View Trash Manager | Positive | Med |`3 Trash DB recs` | 1. Ensure 3 items stored in`.trash/`.<br>2. Open Trash Manager Modal. | Displays all 3 items showing exact structured attributes: Original Path, Name, Deleted Timestamp, Size. | S3 | AC-22.2.1 |
| TC-22-05 | Restore Discarded Mod | Positive | High |`Trash target` | 1. Locate Item in Trash Manager.<br>2. Click "Restore" on item. | Resolves item back to origin accurately ≤ 500ms physically. Re-inserts`folders` DB properties. | S1 | AC-22.2.2 |
| TC-22-06 | Clean Custom Trash DB | Positive | Med |`Items present` | 1. Ensure`.trash/` has items.<br>2. Click "Empty Trash".<br>3. Confirm. | Files physically destroyed carefully (`fs::remove_dir_all`). Sqlite`.trash` schema wiped. | S2 | AC-22.2.3 |
| TC-22-07 | Restore Location Conflict | Negative | High |`Occupied path` | 1. Ensure Trash item origin path is occupied.<br>2. Open Trash Manager.<br>3. Click Restore. | ConflictResolveDialog pops up. Prompt offers Skip or Restore as Copy. Original folder explicitly protected. | S1 | AC-22.2.4 |
| TC-22-08 | Trash Size Warning Boundary | Edge | Med |`5.1GB Trash` | 1. Ensure`.trash/` size exceeds 5GB threshold.<br>2. Open Trash Manager. | Informational UI Warning displays advising cleanup. Not strictly blocked computationally. | S3 | AC-22.2.5 |

## D. Missing / Implied Test Areas

- **Accidental Restore Over Custom Mod Contexts**: If a user switches the active "Game Configuration" and restores an old element blindly, does the backend reject inserting mismatched metadata to the DB? (Implied: Target game ID validated under`Restore` action).
- **Multiple Restorations Running Atomically**: Attempting bulk restore visually inside Trash Manager hitting DB Locks.

## E. Open Questions / Gaps

- "DB record is purged within the same operation". What happens if`trash::delete()` succeeds on the OS side but the`sqlite` transaction immediately fails afterward for an unknown reason? (Implied: The record becomes an orphan DB entry pointing to a missing folder. Next scan cycle purges it).

## F. Automation Candidates

- **TC-22-01 (Soft Delete Core Execution)**: Critical baseline functionality. (Tauri FS mock).
- **TC-22-02 (Robust App Data Trash Failsafe)**: Essential protection feature preventing unrecoverable deletes for cross-drive situations.

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Database mapped to app_data.
- **Context Injection**:
 -`trash::delete()` mock injection required to forcefully simulate cross-drive restrictions.
 - Active File handles spawned locking specific folders simulating 3DMigoto runtime precisely.

## H. Cross-Epic E2E Scenarios

- **E2E-22-01 (Soft Delete DB Cascade Sync)**: User navigates Explorer (Epic 15) and triggers a soft delete (Epic 22). The backend executes a fast move operation to the OS Trash. If OS permissions block the move, the backend returns an explicit error via the Toast system (Epic 36) and aborts the database deletion to prevent state desynchronization.
