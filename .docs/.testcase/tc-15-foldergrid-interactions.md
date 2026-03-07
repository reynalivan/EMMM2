# Test Cases: Explorer Interactions (req-15)

## A. Requirement Summary

- **Feature Goal**: Native contextual interactions including Drag-Marquee (Lasso) selection, Context Menus (right-click), and Drag-and-Drop routing to ObjectList. Also handles Phase 5 context menu additions (Move to Object, Enable Only This, Favorite/Unfavorite sync, Import Thumbnail) and pre-delete warnings.
- **User Stories**:
 - US-15.1: Folder Context Menu
 - US-15.2: Grid Background Context Menu
 - US-15.3: Drag-Marquee (Lasso) Selection
 - US-15.4: Drag-and-Drop to ObjectList Object
- **Success Criteria**:
 - CSS lasso draws bounding mapped rect bounds.
 - Drop layers trace exact cursor mappings routing payloads.
 - Right-click Context interfaces drop standard browser commands for custom Tauri menus.
- **Main Risks**: Browser default right-click context menu overlapping. Lasso selection causing extreme layout recalculation lag on 1000+ items.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-15-foldergrid-interactions.md`

- AC-15.1.1, AC-15.1.2 → TC-15-001
- AC-15.1.3, AC-15.1.4 → TC-15-002
- AC-15.2.1, AC-15.2.2 → TC-15-003
- AC-15.2.3 → TC-15-004
- AC-15.3.1, AC-15.3.2 → TC-15-005
- AC-15.3.3 → TC-15-006
- AC-15.4.1, AC-15.4.2 → TC-15-007
- AC-15.4.3, AC-15.4.4 → TC-15-008
- Phase 5: Move to Object → TC-15-009
- Phase 5: Enable Only This → TC-15-010
- Phase 5: Favorite/Unfavorite → TC-15-011
- Phase 5: pre_delete check warning → TC-15-012

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :---------------------------------------------- | :------- | :------- | :--------------- | :------------------------------------------------------------------------------------- | :--------------- | :----------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------- |
| TC-15-001 | Custom React Context Menu execution | Positive | High | S2 | Mod target`KaeyaMod` available in Grid. |`Mod Target` | 1. Right click explicit Mod boundary visually.<br>2. Verify payload arrays. | Default browser dropdown strictly denied. Component renders custom context menu accurately <50ms. | AC-15.1.1, AC-15.1.2 |
| TC-15-002 | Handling missing interactions context menus | Negative | High | S3 | Invalid clipboard text in memory. |`Text payload` | 1. Connect invalid clipboard text.<br>2. Open Explorer context menu visually. | Interface UI clearly disables contextual options that require images (like "Paste Thumbnail") if validation fails. | AC-15.1.3, AC-15.1.4 |
| TC-15-003 | Background Void UI mapping bounds | Positive | Medium | S3 | Folder Grid open with some blank space below items. |`Blank Space` | 1. Right-click grid void (empty space).<br>2. Target "Refresh". | Menu renders distinct background layout natively (Refresh, Open in Explorer). Refresh triggers cache refetch. | AC-15.2.1, AC-15.2.2 |
| TC-15-004 | Null selection limits | Negative | Low | S4 | Empty Grid (0 items matching query). |`0 Items` | 1. Attempt manual explicit entirely correct specifically structural logically mechanical Select All via shortcut (`Ctrl+A`). | System disables selection logic safely; no invisible boundaries are mapped or arrays accessed. | AC-15.2.3 |
| TC-15-005 | Fluid CSS drag Rect Selection | Positive | High | S2 | Minimum 10 items in Grid. |`valid grid` | 1. Click empty space and drag diagonally across 4 Mod items.<br>2. Release mouse. | Bounding box naturally follows mouse. Intersecting target mapped bounds absolutely appends elements to active selection state. | AC-15.3.1, AC-15.3.2 |
| TC-15-006 | Bounding Auto Scroll limits | Edge | High | S3 | 50+ items in Grid (requires vertical scrolling). |`Long List` | 1. Start Drag Marquee selection.<br>2. Drag cursor down near bottom bounds of the Grid container. | Engine adjusts scroll downwards while maintaining selection box logic mechanically. | AC-15.3.3 |
| TC-15-007 | Structural Drag Payload targets | Positive | High | S2 | 5 targets selected. Sidebar ObjectList visible. |`5 targets` | 1. Click and hold the selection.<br>2. Drag mapping to explicitly specific droppable boundary on the ObjectList (e.g., "Albedo").<br>3. Release. | Drop routes payloads. React state updates visually. Backend receives command to move 5 targets to "Albedo". | AC-15.4.1, AC-15.4.2 |
| TC-15-008 | Logical Droppable Rejection handling explicitly | Negative | High | S3 | 1 target selected. |`Illegal zone` | 1. Drag target item.<br>2. Hover over non-droppable area (e.g., Settings icon).<br>3. Release cursor. | Element drops logic accurately returning. No backend action is taken. Drag state is cleared. | AC-15.4.3, AC-15.4.4 |
| TC-15-009 | Context Menu: Move to Object | Positive | High | S2 |`KaeyaMod` selected. Target Object`Venti` exists. | Node:`KaeyaMod` | 1. Right-click`KaeyaMod`.<br>2. Hover "Move to..."<br>3. Select`Venti` from the submenu.<br>4. Wait for execution.<br>5. Check physical disk. | Folder physically moved to`Venti` object folder. UI immediately removes it from current grid and recalculates ObjectList counts. | Phase 5 |
| TC-15-010 | Context Menu: Enable Only This | Positive | High | S2 |`KaeyaMod` selected (disabled). 3 other mods in the same Object are currently enabled. | Node:`KaeyaMod` | 1. Right-click`KaeyaMod`.<br>2. Select "Enable Only This".<br>3. Observe UI updates. |`KaeyaMod` is enabled (prefix removed). The 3 previously enabled mods are disabled (prefix added). All actions happen atomically under OperationLock. UI syncs. | Phase 5 |
| TC-15-011 | Context Menu: Favorite/Unfavorite sync | Positive | High | S3 |`KaeyaMod` selected. Not favorited. | Node:`KaeyaMod` | 1. Right-click`KaeyaMod`.<br>2. Click "Add to Favorites".<br>3. Observe Grid card.<br>4. Observe`info.json`. | Grid card instantly displays star icon badge. Backend task writes`"favorite": true` into`info.json` strictly preserving other fields. | Phase 5 |
| TC-15-012 | Context Menu: pre_delete check count warning | Edge | High | S2 |`KaeyaMod` selected. Contains nested objects inside (e.g., 5 INIs). | Node:`KaeyaMod` | 1. Right-click`KaeyaMod`.<br>2. Click "Delete".<br>3. Observe custom dialog. | The delete confirmation dialog intercepts the flow and issues a warning incorporating the pre-delete item count (e.g., "This folder contains 5 configuration files... are you sure?"). | Phase 5 |

## D. Missing / Implied Test Areas

- **Paste from Web Browser**: Does copying an image _from Google Images_ functionally place raw bytes matching OS structural clipboards, or does it specifically copy entirely unmapped HTML ? (Needs verification mechanically).

## E. Open Questions / Gaps

- "Lasso selection onMouseDown" -> Does capturing mouse actively block explicit scroll operations ?

## F. Automation Candidates

- **TC-15-001 (Custom Right-Click Event Hijack)**: Playwright evaluating browser default event accurately mapping visually checking exact component.
- **TC-15-005 (Rectangle logic mapping accurately)**: Testing computing boundaries exactly via layout coordinates.

## G. Test Environment Setup

- **Preconditions**: OS: Windows 10/11. App: EMMM2 dev build. Mouse + Keyboard setup connected accurately simulating events.
- **Context Injection**:
 -`mods_path/Characters/Kaeya/` with > 100 entries rendering enabling scrolling behaviors.

## H. Cross-Epic E2E Scenarios

- **E2E-15-01 (Drag & Drop Hierarchy Organization)**: User executes Drag-Marquee Selection exactly matching target bounding logic (Epic 15) isolating 10 entries. Then dragging into the ObjectList (Epic 06) safely dropping payloads updating disk paths updating active lists.
