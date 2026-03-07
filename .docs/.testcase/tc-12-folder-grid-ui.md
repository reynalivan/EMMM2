# Test Cases: Folder Grid UI (req-12)

## A. Requirement Summary

- **Feature Goal**: High-performance rendering wrapper connecting`list_folders` array outputs logically manipulating visual representation (Virtualization, List vs Grid modes, sorting locally, breadcrumb routing).
- **User Stories**:
 - US-12.1: Grid vs List Modes
 - US-12.2: Sorting the Grid
 - US-12.3: Breadcrumb Navigation
 - US-12.4: Empty States
- **Success Criteria**:
 - 1000 items map at 60fps utilizing`@tanstack/react-virtual`.
 - View toggles process under 100ms.
 - Sorting calculations map logically bounding 50ms metrics purely client-side without re-fetching payloads.
 - Custom Empty States render appropriately mapping specific context logic 0 visual glitches.
- **Main Risks**: Sorting logic crashing natively during missing timestamp validations (OS inconsistencies causing`NaN`). Resize Observers failing virtualization bounds throwing element layouts totally outside window margins horizontally.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-12-folder-grid-ui.md`

- AC-12.1.1, AC-12.1.2 → TC-12-01
- AC-12.1.3 → TC-12-02
- AC-12.1.4 → TC-12-03
- AC-12.2.1, AC-12.2.2 → TC-12-04
- AC-12.2.3, AC-12.2.4 → TC-12-05
- AC-12.3.1, AC-12.3.2 → TC-12-06
- AC-12.3.3 → TC-12-07
- AC-12.3.4 → TC-12-08
- AC-12.4.1, AC-12.4.2 → TC-12-09
- AC-12.4.3 → TC-12-10

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | -------------------------------------- | -------- | -------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-12-01 | View Layout Logic switch + Persistence | Positive | High | Standard Payload | 1. Toggle Layout view (Grid -> List).<br>2. Restart App.<br>3. Verify Layout state. | Layout manipulates DOM bounds effectively under 100ms. Settings persist inside browser`localStorage` and load directly next session. | S2 | AC-12.1.1, AC-12.1.2 |
| TC-12-02 | Scroll Position Mapping recovery | Negative | Medium | 1000 items list | 1. Scroll deep within List.<br>2. Switch layout explicitly midway down scroll bounds.<br>3. Check Y-Offset. | Reset Y-bounds mappings to 0 origin ensuring element bounds don't visually trap themselves outside rendering boxes. | S3 | AC-12.1.3 |
| TC-12-03 | Column layout resizing evaluation | Edge | Medium | 20 entries | 1. Activate Grid.<br>2. Rapidly adjust window scaling dimensions horizontally.<br>3. Inspect Grid Reflow. | React virtualized constraints resize inside single frame render metrics avoiding overflow/clip text bugs. | S3 | AC-12.1.4 |
| TC-12-04 | Sorting parameter efficiency | Positive | High | Heavy DOM load (>1000 items) | 1. Inject >1000 Active mods.<br>2. Trigger Sort switching (A-Z -> Modified).<br>3. Profile Action Speed. | Operation filters array internally bypassing backend refetch resolving visual mapping ≤50ms. Parameter persists visually. | S2 | AC-12.2.1, AC-12.2.2 |
| TC-12-05 | Handling Null/Missing Metadata sorts | Negative | High | Elements missing timestamps | 1. Load array containing missing OS timestamp values.<br>2. Evaluate Date Modified sorting parameter.<br>3. Asses Sort Logic. | Execution pushes undefined variables precisely against array bounds without yielding sorting exceptions/crashes. | S2 | AC-12.2.3, AC-12.2.4 |
| TC-12-06 | Standard Breadcrumb navigation | Positive | High |`Characters/Albedo/Skin` | 1. Navigate into Sub-folder hierarchy 3 nodes deep.<br>2. Click "Characters" directly in breadcrumb.<br>3. Analyze routing. | Visually maps segment strings. Click routes Zustand payload popping query via cache ≤200ms. | S2 | AC-12.3.1, AC-12.3.2 |
| TC-12-07 | Illegal Path navigation | Negative | Low | Missing OS Path string | 1. Manually insert invalid breadcrumb mapping externally into Zustand state.<br>2. Observe App reaction. | Screen draws specifically clear mapped Component (`Folder not found`) instead of freezing system logic handling string exception. | S3 | AC-12.3.3 |
| TC-12-08 | Contextual Empty states | Positive | High | Valid missing data contexts | 1. Open null physical Folder.<br>2. Execute Search yielding 0 results.<br>3. Compare explicitly specific drawing contexts. | Natively distinguishing structurally specific components (Empty Directory vs Search Null) yielding responsive visual feedback. | S3 | AC-12.4.1, AC-12.4.2 |
| TC-12-09 | Fast transition states | Edge | Low | Large payload fetch | 1. Trigger Active mapped Background query population logic.<br>2. Scan fetches parameters. | Mapping drops empty state bounding specific Component logic accurately under 200ms without glitch visual delays. | S3 | AC-12.4.3 |
| TC-12-10 | Node-type Navigation execution | Positive | High | 1 ModPackRoot, 1 Container | 1. Double click`ContainerFolder` node.<br>2. Go back.<br>3. Double click`ModPackRoot` node.<br>4. Inspect Preview Panel / routing. | Clicking`ContainerFolder` changes directory layout. Clicking`ModPackRoot` explicitly does NOT change directory, but sets`activeModId` focusing the details panel. | S1 | US-12.2 |
| TC-12-11 | Variant Picker modal trigger | Positive | High | 1`VariantContainer` node | 1. Configure workspace housing a`VariantContainer` node.<br>2. Double click the Variant Item card.<br>3. Look for modal popup.<br>4. Select a variant sub-folder from the modal. | The Variant Picker overlay renders immediately blocking the screen. Selecting an option changes physical folder name, swapping the variant, and the toast indicates success. Grid updates state automatically. | S1 | US-12.5 |
| TC-12-12 | Context menu variations by type | Positive | Medium | 3 different node types | 1. Right click`ContainerFolder`.<br>2. Close.<br>3. Right click`ModPackRoot`.<br>4. Close.<br>5. Right click`VariantContainer`. |`ContainerFolder` shows Open in Explorer.`ModPackRoot` additionally shows Enable/Disable/Favorite/Move.`VariantContainer` shows exclusively 'Change Variant...' without Enable/Disable options. | S2 | US-12.6 |
| TC-12-13 | Visual language badges and tooltips | Positive | Low |`VariantContainer`, No-Thumb | 1. Look at`VariantContainer` thumbnail in Grid mode.<br>2. Look at a folder missing`preview.png`.<br>3. Hover over the title of any folder that exceeds text limits. |`VariantContainer` visually displays a "V" badge. Missing thumbnail displays a gradient fallback with folder icon. Hovering truncated title triggers a full native tooltip. | S3 | AC-12.1.X |
| TC-12-14 | Within-group sort rule | Edge | High | 2 ModRoot, 2 Container folders | 1. Ensure view holds`ModPackRoot` items and`ContainerFolder` items.<br>2. Change Sort order.<br>3. Verify sequence logic physically. | Regardless of A-Z or Modified constraints, ALL`ContainerFolder` items are sorted securely to the TOP of the list, identical to standard Windows Explorer paradigm.`ModPackRoot` logic sinks below. | S2 | US-12.2 |
| TC-12-15 | Advanced mode breadcrumb | Positive | Medium | Deep directory,`Alt` key | 1. Navigate`Objects/Characters/Albedo`.<br>2. Hold`Alt` and hover the Breadcrumb.<br>3. Click the absolute filesystem link generated. | The visual`/` delimited list converts to a physical`C:\Empath...\Albedo` absolute raw path on hover. Clicking raw path opens Windows Explorer instantly to that target. | S3 | US-12.3 |

## D. Missing / Implied Test Areas

- **Keyboard Focus in Virtualizer**: Virtualized lists often break native`<Tab>` or Arrow key browser scrolling if not explicitly handled (e.g. focused element gets unmounted when scrolled out of view).

## E. Open Questions / Gaps

- "Sort by Date Modified" client-side. Date Modified refers to OS modification. Does`list_folders` attach`modified` timestamp onto exactly`FolderEntry`? It wasn't specifically mentioned in Epic 11's data structure but is required here for Epic 12 to do client-side sorting.

## F. Automation Candidates

- **TC-12-04 (Client-Side Sorting without Refetch)**: E2E testing using DOM observation natively assessing that when sort buttons trigger, no IPC or HTTP equivalent request arrays are generated against backend routes.
- **TC-12-08 (Empty State accuracy)**: React component testing validating explicit string outputs without network mocked lag.

## G. Test Environment Setup

- **OS**: Windows 11
- **UI State**: Redux/Zustand devtools installed to monitor visual layouts.
- **FileSystem Data**:
 -`ContainerFolder` x2
 -`ModPackRoot` x1001 (for virtualizer stretch test)
 -`VariantContainer` x1 (for modal popping)

## H. Cross-Epic E2E Scenarios

- **E2E-12-01 (Explorer Selection Sorting)**: Folder List (Epic 11) structurally fetches >1000 items injecting visually rendering Grid (Epic 12) instantly mapped into Smart Sort Filters (Epic 08) manipulating active Selection bounds manually (Epic 15) securely without re-fetch loops.
- **E2E-12-02 (Node Click to Details)**: Double click`ModPackRoot` passing payload to`activeModId`. Ensure Preview Panel Layout (Epic 16) populates`info.json` strictly without dropping frame-rates or rendering ghost data.
