# Test Cases: Preview Panel Layout & State (Epic 16)

## A. Requirement Summary

- **Feature Goal**: Manage right-panel state logic (Object Summary vs Single Mod vs Multi-Select) based on the grid selection, while preserving width configuration. Includes Phase 5 additions like the Large Enable/Disable toggle in the header.
- **User Roles**: End User
- **User Story**:
 - US-16.1: Dual Context Rendering
 - US-16.2: Resizable & Collapsible Panel
 - US-16.3: Multi-Selection State
- **Success Criteria**:
 - Panel transitions in ≤ 100ms.
 - Collapse/expand animation ≤ 200ms.
 - Width restored from`localStorage` in ≤ 50ms.
 - Switching games clears selected folders.
 - Deleted entries revert the layout back to Object Summary within ≤ 200ms.
- **Main Risks**: Stale caches causing ghost UI panels throwing`undefined` exceptions, or React Resizable breaking bounds and trapping the panel in a 0px width.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-16-preview-panel-layout.md`

- AC-16.1.1 → TC-16-001
- AC-16.1.2 → TC-16-002
- AC-16.1.3 → TC-16-003
- AC-16.1.4 → TC-16-004
- AC-16.1.5 → TC-16-005
- AC-16.2.1 → TC-16-006
- AC-16.2.2 → TC-16-007
- AC-16.2.3 → TC-16-008
- AC-16.3.1 → TC-16-009
- AC-16.3.2 → TC-16-010
- Phase 5: Large Enable/Disable toggle → TC-16-011

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :--------------------------- | :------- | :------- | :--------------- | :------------------------------------------------------------------------------ | :------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------- |
| TC-16-001 | Default Object Summary State | Positive | High | S2 | App is open. Object`Kaeya` is selected in ObjectList. No mod selected in Grid. |`No grid selection` | 1. Ensure Object is selected in ObjectList.<br>2. Clear any Mod selections using`Esc`.<br>3. Observe Preview Panel. | Panel shows`ObjectSummary` with object name ("Kaeya"), total mod count, enabled count, and optional 3D render/thumbnail. | AC-16.1.1 |
| TC-16-002 | Single Mod Selection | Positive | High | S2 | App is open. Mod`KaeyaMod_1` is visible in Grid. |`1 Mod Target` | 1. Click the`KaeyaMod_1` folder card in Grid.<br>2. Observe Preview Panel transition. | Panel switches to`ModDetails` view in ≤ 100ms. Shows preview image, name, and attributes for`KaeyaMod_1`. | AC-16.1.2 |
| TC-16-003 | Deselecting Mod | Positive | High | S3 |`KaeyaMod_1` is currently selected in Grid. |`N/A` | 1. Click empty space in the grid.<br>2. Observe Preview Panel. |`selectedFolders` state array resets to empty`[]`. Panel reverts to`ObjectSummary` in ≤ 100ms. | AC-16.1.3 |
| TC-16-004 | External Deletion Handling | Negative | High | S2 |`KaeyaMod_1` is selected. |`Target Mod` | 1. Select the Mod.<br>2. Delete the physical folder`KaeyaMod_1` using Windows Explorer.<br>3. Wait for FileWatcher trigger. | Panel reverts to`ObjectSummary` automatically without crashing. The folder grid removes the deleted item. | AC-16.1.4 |
| TC-16-005 | Game Context Switch | Edge | Medium | S3 |`KaeyaMod_1` is selected. Multiple games available in TopBar. |`Another Game` | 1. Keep Mod selected.<br>2. Switch the active game from`Genshin` to`ZZZ` via TopBar.<br>3. Observe Panel. |`selectedFolders` clears. Panel shows the new game's default`ObjectSummary` or empty state. | AC-16.1.5 |
| TC-16-006 | Panel Resizing Bounds | Positive | High | S3 | App is open. |`N/A` | 1. Drag the vertical divider handler to the left and right limits.<br>2. Restart App.<br>3. Verify panel width upon reload. | Width updates. UI limits dragging (min width: 240px, max: 50% of viewport). The saved width value persists in localStorage across restarts. | AC-16.2.1 |
| TC-16-007 | Panel Collapse/Expand | Positive | Medium | S4 | App is open. |`N/A` | 1. Click the collapse arrow icon next to the divider.<br>2. Click it again to expand.<br>3. Verify animation speed. | Panel collapses to a minimized strip. Expected subjective delay ≤ 200ms. Clicking expand restores previous exact width. | AC-16.2.2 |
| TC-16-008 | OS Window Resize Crushing | Edge | Medium | S3 | App runs in windowed mode (not maximized). |`N/A` | 1. Resize OS application window horizontally to be narrower than Panel min width + Grid min width.<br>2. Observe Panel. | Panel collapses automatically (or scales responsively) to protect the main Grid view. Layout does not break or overflow out of bounds. | AC-16.2.3 |
| TC-16-009 | Multi-Selection Placeholder | Positive | High | S2 | Minimum 3 mods available in Grid. |`3 Mods` | 1. Hold`Ctrl` and click 3 distinct mods.<br>2. Observe Panel. | Panel shows bulk action view ("3 items selected") with appropriate icons (e.g., bulk toggle, bulk delete). Individual metadata is hidden. | AC-16.3.1 |
| TC-16-010 | Multi-Selection to Single | Edge | Medium | S3 | 3 mods currently selected. |`N/A` | 1. Hold`Ctrl` and click 2 of the currently selected mods to deselect them.<br>2. Observe Panel. | As soon as selection count drops to 1, Panel transitions to`ModDetails` view for that single remaining item in ≤ 100ms. | AC-16.3.2 |
| TC-16-011 | Header Toggle Switch | Positive | High | S2 | 1 disabled mod selected. |`N/A` | 1. Select the disabled mod.<br>2. Click the large Enable/Disable toggle directly in the preview panel header.<br>3. Verify backend and Grid state. | Target mod gets enabled via IPC. The Grid card instantly updates cheerfully and the panel header toggle switches to the active/green state. UI remains responsive. | Phase 5 |

## D. Missing / Implied Test Areas

- **Empty State on Startup**: What happens when no object is selected in the objectlist yet? (Implied: the panel is empty or shows a global app summary).
- **Minimum Width Contents Layout**: When the panel is exactly at min size (240px), does the text overflow or wrap without breaking the UI?

## E. Open Questions / Gaps

- "Collapse animation ≤200ms":`react-resizable-panels` standard collapse is instant. Does the design system include custom CSS transitions to handle this constraint?

## F. Automation Candidates

- **TC-16-001 / TC-16-002 / TC-16-003**: Core navigation flow, highly stable and critical (Playwright/Vitest).
- **TC-16-009 / TC-16-010**: Multi-select transitions are crucial for state management logic validation (Vitest).

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. ObjectList instantiated tracking Object counts. Grid rendering >= 10 mod objects.
- **Context Injection**:
 - Valid`info.json` populated with specific keys (Name, Version, Author) rendering inside the detail panes visually.

## H. Cross-Epic E2E Scenarios

- **E2E-16-001 (Selection Context Resets)**: User clicks active Mod in Explorer Grid (Epic 15) populating specific Preview Panel payload (Epic 16). User triggers Game Context Switch directly linking through TopBar (Epic 04). Active ObjectList Object implicitly updates (Epic 10) resetting the preview contextual hook, effectively clearing memory and preventing stale UI states.
