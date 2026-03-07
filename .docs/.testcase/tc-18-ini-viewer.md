# Test Cases: INI Viewer & Editor (Epic 18)

## A. Requirement Summary

- **Feature Goal**: Allow users to browse and edit`.ini` files directly in the app with proper 3DMigoto syntax highlighting. Includes "Unsaved" state tracking and collision avoidance via`WatcherSuppression`.
- **User Roles**: End User
- **User Story**:
 - US-18.1: Discover INI Files
 - US-18.2: Syntax Highlighting
 - US-18.3: Direct Editing & Save
- **Success Criteria**:
 -`list_mod_ini_files` completes in ≤ 100ms.
 - Content loads in ≤ 200ms for ≤ 500KB files.
 - Custom CodeMirror 6 highlighting captures 95% of 3DMigoto syntax.
 -`write_mod_ini` saves in ≤ 300ms, using`WatcherSuppression`.
 - Content not saved if exactly matching loaded text ("dirty" detection).
- **Main Risks**: Accidental overwriting of locked INI files causing exceptions. Saving INI without`WatcherSuppression` triggering an endless filesystem reload loop. Path traversal via malicious INI filenames.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-18-ini-viewer.md`

- AC-18.1.1 → TC-18-01
- AC-18.1.2 → TC-18-02
- AC-18.1.3 → TC-18-03
- AC-18.2.1, AC-18.2.2 → TC-18-04
- AC-18.2.3 → TC-18-05
- AC-18.3.1 → TC-18-06
- AC-18.3.2 → TC-18-07
- AC-18.3.3 → TC-18-08
- AC-18.3.4 → TC-18-09
- Implied Path Traversal → TC-18-10

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | --------------------------------- | -------- | -------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-18-01 | List INI Files | Positive | High |`5 INI files` | 1. Expand INI section for a mod known to have`.ini` files. | Dropdown populates all 5 files in ≤ 100ms. | S3 | AC-18.1.1 |
| TC-18-02 | Load INI Content | Positive | High |`1 INI file` | 1. Select the file from the dropdown. | Content loads into the CodeMirror editor in ≤ 200ms. | S2 | AC-18.1.2 |
| TC-18-03 | No INI Files UI | Negative | Med |`N/A` | 1. Expand INI section for a mod with no`.ini` files. | Shows "No INI files found." The app does not crash or show an empty dropdown. | S3 | AC-18.1.3 |
| TC-18-04 | Valid Syntax Highlighting | Positive | High |`INI with key/value, comment, headers` | 1. Load an INI file.<br>2. Inspect editor content visually. | Specific colors match 3DMigoto dialect: section headers (`[...]`), keys, variables (`$val`), and comments (`;`). | S3 | AC-18.2.1, AC-18.2.2 |
| TC-18-05 | Invalid Syntax Fallback | Edge | Low |`[]`, duplicate keys | 1. Load an odd / syntactically incorrect INI file. | Editor renders plain text, no crashes occur. | S4 | AC-18.2.3 |
| TC-18-06 | Editor Dirty State | Positive | High |`"New Value"` | 1. Load an INI file.<br>2. Type inside the editor. | Save button appears, "Unsaved changes" indicator shows. | S3 | AC-18.3.1 |
| TC-18-07 | Execute File Save | Positive | High |`Ctrl+S Keypress` | 1. Create unsaved changes.<br>2. Press Ctrl+S. | Saves in ≤ 300ms.`WatcherSuppression` triggers. Unsaved indicator disappears. | S2 | AC-18.3.2 |
| TC-18-08 | Locked File Rejection | Negative | High |`"New Value"` | 1. Make a change in the editor.<br>2. Lock the`.ini` file with another process.<br>3. Click Save. | A "Save failed: file locked" toast appears. The editor retains unsaved content. | S2 | AC-18.3.3 |
| TC-18-09 | Navigation While Unsaved | Edge | Med |`N/A` | 1. Create unsaved changes.<br>2. Select a different Mod from the Grid. | A confirmation dialog "Discard changes?" blocks immediate navigation. | S2 | AC-18.3.4 |
| TC-18-10 | Malicious Filename Path Traversal | Negative | High |`../../../Windows/System32/config.ini` | 1. Intercept / forge request to read/write specific out-of-bounds`fileName` via`read_mod_ini` /`write_mod_ini` API. | Backend validation strictly denies reads/writes. Errors safely protecting the host system. | S1 | Implied |

## D. Missing / Implied Test Areas

- **External File Changes**: What happens when an INI is edited in Notepad++ and saved while open in EMMM2? (Implied: File watcher event invalidates the cache, Editor should prompt to reload or auto-reload if not dirty).
- **Encoding Issues**: What if the INI file uses an unusual encoding like Shift-JIS or UTF-16? (Implied: Standardized to UTF-8 or graceful failure gracefully).

## E. Open Questions / Gaps

- "Save only supports whole-file overwrite". If a user opens a 2MB INI file, edits 1 character, it overwrites the whole 2MB file. Does this affect performance or disk lifespan significantly?

## F. Automation Candidates

- **TC-18-01 / TC-18-02**: Reading INI datasets and ensuring performance specs are met. (Tauri e2e).
- **TC-18-06 / TC-18-09**: Unsaved state transitions and guard clauses blocking page unload/navigation (Vitest).
- **TC-18-10**: Security path traversal guard verification (Tauri Unit Testing).

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. File Watcher active. CodeMirror syntax package installed.
- **Context Injection**:
 - Valid`desktop.ini` or custom`mod.ini` created physically via OS simulating a valid target.

## H. Cross-Epic E2E Scenarios

- **E2E-18-01 (Watcher Suppression Flow)**: User selects a Mod via the Preview Panel (Epic 16) and loads its`mod.ini` in the Editor Viewer (Epic 18). They type out new variables and press Save. The backend issues a WatcherSuppression token (Epic 28) to prevent the File Watcher from triggering a reload event, ensuring the React UI state and scroll bounds are preserved.
