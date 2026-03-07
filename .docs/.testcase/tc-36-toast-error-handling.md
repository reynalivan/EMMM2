# Test Cases: Toast & Error Handling Pipeline (Epic 36)

## A. Requirement Summary

- **Feature Goal**: Provide a consistent, non-intrusive error handling surface across EMMM2. This involves a`useToastStore` for background operations, a React`ErrorBoundary` for rendering crashes, and a rollback-aware`CommandError` system returning serialized JSON instead of silent Rust panics.
- **User Roles**: Application User, Power User / Developer.
- **User Story**: As a user, I want clear, actionable pop-up messages when things succeed or fail, and a graceful recovery screen if the app crashes, instead of confusing white screens or raw technical backtraces.
- **Acceptance Criteria**:
 - Success toasts auto-dismiss in 3s; Error toasts require explicit dismissal.
 - In-progress spinners are replaced natively by success/error states upon mutation completion.
 -`ErrorBoundary` isolates crashes, rendering a fall-back UI with`componentStack` and a Reload button, avoiding full app unmounts where possible.
 - Built-in UI to view the last 200 lines of`emmm2.log` in`<500ms`, plus an "Open Log Folder" button.
- **Success Criteria**: 0 unhandled promise rejections on IPC invoke calls. Error logs never expose PIN hashes or raw UI DOM strings.
- **Main Risks**: Stacking too many toasts overflowing the viewport; Error boundary failing to mount causing white screens.
## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :--------------------------------- | :---------------- | :------------------------------------------------------------------ |
| AC-36.1.1 (Success Auto-dismiss) | TC-36-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.1.2 (Error Explicit-dismiss) | TC-36-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.1.3 (In-progress Spinner) | TC-36-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.1.4 (Stacking Limit) | TC-36-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.2.1 (App Crash Fallback) | TC-36-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.2.2 (Error Log Writing) | TC-36-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.2.3 (Sub-tree Isolation) | TC-36-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.3.1 (Log Viewer UI) | TC-36-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.3.2 (Open Folder Shell) | TC-36-009 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |
| AC-36.3.3 (Missing Log Safety) | TC-36-010 |`e:\Dev\EMMM2NEW\.docs\requirements\req-36-toast-error-handling.md` |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :------------------------------ | :------- | :------- | :---------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :---------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-36-001 | Success Auto-dismiss | Positive | High | Any DB mutate | 1. Ensure Valid Action Ready.<br>2. Trigger a successful DB mutation (e.g., mark mod as favorite).<br>3. Observe UI bottom-right corner.<br>4. Start stopwatch.<br>5. Wait for dismissal. | Green toast appears: "Mod favorited successfully". Toast disappears automatically after exactly 3000ms. | S3 | AC-36.1.1 |
| TC-36-002 | Error Explicit-dismiss | Positive | High | File access lock | 1. Simulate Error condition.<br>2. Lock a mod's`info.json` file.<br>3. Attempt to rename the mod.<br>4. Observe UI.<br>5. Wait 5s.<br>6. Click the '✕' button. | Red toast appears displaying error message (e.g., "Permission denied"). Toast stays on screen indefinitely (>5s) until '✕' is clicked. | S2 | AC-36.1.2 |
| TC-36-003 | In-progress Spinner transitions | Positive | Medium | 10s sleep mock | 1. Ensure action requiring >500ms.<br>2. Inject a 2s delay into a mutation command.<br>3. Trigger the mutation.<br>4. Observe the toast area.<br>5. Wait for completion. | "In-Progress" spinner toast appears immediately. Upon completion, the spinner toast unmounts and is replaced instantly by the Success toast, without duplicating. | S3 | AC-36.1.3 |
| TC-36-004 | Flood Control Cap | Edge | Low | 15 fail queries | 1. Setup fast clicking loop.<br>2. Trigger 15 rapid, simultaneous error-producing actions (e.g., spam clicking a failing action).<br>3. Count the visible toasts on screen.<br>4. Check browser console logs. | UI caps at 5 stacked toasts max visually. The 10 remaining dropped errors are logged to the console without crashing the UI. | S3 | AC-36.1.4 |
| TC-36-005 | Full React Crash Fallback | Positive | High |`throw new Error()` | 1. Setup UI Mock.<br>2. Inject an unhandled exception into the root DOM render (e.g.,`throw new Error('Test Crash')`).<br>3. Observe the screen. | App does not white-screen. Renders ErrorBoundary fallback UI: "Oops, something went wrong", displays stack trace, and provides a functional "Reload App" button. | S1 | AC-36.2.1 |
| TC-36-006 | Error Boundary Log Sync | Positive | Medium | Log viewer open in tail | 1. Ensure UI crashed.<br>2. Crash app UI component as in TC-36-005.<br>3. Open`emmm2.log` on disk.<br>4. Search for the error string. | The exact JS exception string ('Test Crash') is mirrored securely into`emmm2.log` via Tauri plugin automatically. | S2 | AC-36.2.2 |
| TC-36-007 | Panel Crash Isolation | Positive | High | Bad Grid Node | 1. Ensure Grid component mock crash.<br>2. Inject a render exception strictly inside the`FolderGrid` component.<br>3. Observe UI layout.<br>4. Attempt to use ObjectList and Topbar. |`FolderGrid` area displays localized fallback UI. ObjectList and Topbar remain 100% responsive and unblocked. Mod navigation still works. | S2 | AC-36.2.3 |
| TC-36-008 | View last 200 lines | Positive | Medium | System Log text | 1. Ensure`emmm2.log` has 1000 lines.<br>2. Generate a 1000-line`emmm2.log` file.<br>3. Navigate to Settings > Logs.<br>4. Count the rendered lines.<br>5. Check render performance. | Renders exactly the last 200 lines inside`<500ms`. Rendered in a`<pre>` monospace block. | S3 | AC-36.3.1 |
| TC-36-009 | Shell Open directory | Positive | Low | Log paths | 1. Ensure OS Desktop context.<br>2. Navigate to Settings > Logs.<br>3. Click "Open Log Folder" button.<br>4. Observe OS behavior. | Tauri IPC strictly launches system File Explorer focused on the`app_data_dir/logs/` directory containing`emmm2.log`. | S3 | AC-36.3.2 |
| TC-36-010 | Missing File Tolerance | Negative | Medium | Deleted`emmm2.log` | 1. Ensure fresh un-launched state.<br>2. Delete`emmm2.log` while the app is closed.<br>3. Launch the app.<br>4. Navigate to Settings > Logs. | UI gracefully states "No logs yet". No Rust panic, stable rendering. | S3 | AC-36.3.3 |

## D. Missing / Implied Test Areas

- **Promise Handler Coverage**: Ensure that _all_ Tauri`invoke` calls throughout the React codebase possess`.catch()` or`onError` wrappers forwarding responses specifically to the Zustand toast store.
- **PIN Log Safety**: Ensure that when entering a Safe Mode PIN incorrectly, the raw PIN or hash is NOT logged to`emmm2.log` inside the error string.

## E. Open Questions / Gaps

- When the maximum toast limit (5) is reached, does the oldest toast get pushed out (FIFO), or are new toasts discarded until a slot opens?

## F. Automation Candidates

- **TC-36-004 (Flood limit)**: React Testing Library — dispatch`addToast` 15 times, assert DOM query selector finds exactly 5 toast elements.
- **TC-36-005 (React Crash)**: Vitest component mount — forcefully throw inside a child, assert`ErrorBoundary`'s`componentDidCatch` updates state and fallback tree renders.
- **TC-36-008 (Log limits)**: Rust unit test — write 1000 lines to mock file, call`read_logs` command, assert returned Vec length == 200.

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **DB State**:`emmm2.db` (any valid state)
- **Zustand Store**: toast store initialized with default limit (5)
- **Log Path**:`%APPDATA%/EMMM2/logs/emmm2.log`

## H. Cross-Epic E2E Scenarios

- **E2E-36-01 (Bulk Operation Failure Visibility)**: Select 500 mods spanning 10 different games in the`FolderGrid` (Epic 11). Execute a "Delete" operation (Epic 22). During the operation, intentionally revoke file-system permissions for 5 random mod folders. Verify that the UI presents a multi-line, scrollable error Toast aggregating the 5 specific folder path failures (`S2`), whilst reporting the 495 successes. Verify that precisely 5 error messages are logged securely to`emmm2.log` (Epic 36) without leaking any sensitive environment data or crashing the background asynchronous loop.
- **E2E-36-02 (Safe Mode PIN Evaluation and Error Logging Integration)**: Activate Safe Mode (Epic 30). Navigate to a restricted NSFW mod and attempt to edit its`info.json` (Epic 17). Enter an incorrect PIN. Verify that the Error Toast strictly displays a generic "Invalid PIN" message. Crucially, open the Settings > Logs viewer (Epic 36) and verify that the specific inputted incorrect PIN string is ABSOLUTELY NOT present in the logged`emmm2.log` output, guaranteeing zero-leakage security boundaries for password attempts`S1`. Ensure that closing the toast restores the "Unlock" prompt.
