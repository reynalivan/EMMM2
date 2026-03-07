# Test Cases: ObjectList Navigation & Resizing (req-06)

## A. Requirement Summary

- **Feature Goal**: Manage rendering of the ObjectList to browse GameSchema parsed categories as virtualized lists. It must safely shrink into a responsive drawer overlay on mobile viewports (< 768px).
- **User Stories**:
 - US-06.1: Resizable ObjectList Layout
 - US-06.2: Category Sections
 - US-06.3: Mobile / Responsive Adaptation
- **Success Criteria**:
 - 60fps drag render (≤ 16ms/frame).
 - Width limits maintained: 180px min, 600px max.
 - Category collapse with 500+ items triggers in ≤ 100ms.
 - Viewports < 768px pop out the component as a `fixed` drawer in ≤ 100ms.
- **Main Risks**: Excessive DOM rendering of massive uncollapsed categories blocking main thread, UI overlap blocking clicks in drawer mode.

## B. Coverage Matrix

**Requirement File**: `e:\Dev\EMMM2NEW\.docs\requirements\req-06-objectlist-navigation.md`

- AC-06.1.1, AC-06.1.2 → TC-06-01
- AC-06.1.3 → TC-06-02
- AC-06.1.4 → TC-06-03
- AC-06.2.1, AC-06.2.2 → TC-06-04
- AC-06.2.3 → TC-06-05
- AC-06.2.4 → TC-06-06
- AC-06.3.1, AC-06.3.2 → TC-06-07
- AC-06.3.3 → TC-06-08
- AC-06.3.4 → TC-06-09

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ----------------------------------- | -------- | -------- | --------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------- | -------------------- |
| TC-06-01 | Resize & Persist ObjectList | Positive | High | Workspace Mounted | 1. Drag the ObjectList right 50px.<br>2. Refresh Application. | Resize renders at ≥60fps. Restored width reads fractional percentage from localStorage in ≤50ms. | S2 | AC-06.1.1, AC-06.1.2 |
| TC-06-02 | Absolute Boundary Clashing | Negative | High | Mouse Drag | 1. Drag ObjectList left fully (0px).<br>2. Drag ObjectList right (800px). | Width bounds strictly cap at 180px and 600px. Content does not clip or hide. | S3 | AC-06.1.3 |
| TC-06-03 | Squeeze Panel Window Size | Edge | Med | OS Window Scale | 1. Resize OS window below 1024px. | ObjectList stops compressing precisely at 180px and remains visible. | S4 | AC-06.1.4 |
| TC-06-04 | Category Toggle & Memory | Positive | High | Valid Parsed UI | 1. Click chevron on 'Characters' group header.<br>2. Restart App. | Group toggles in ≤ 100ms. Collapse boolean is stored in `sidebarCategoryStates` and restored. | S3 | AC-06.2.1, AC-06.2.2 |
| TC-06-05 | Safe Category Fallback | Negative | High | Missing Enum schema | 1. Inject an object matching no existing schema category. | Object renders inside a fallback "Uncategorized" header block. | S2 | AC-06.2.3 |
| TC-06-06 | Massive Scale Collapse Render | Edge | High | 500+ Character Set | 1. Construct database with 500 distinct objects.<br>2. Expand target category. | Expand action completes in ≤ 100ms. List renders using `@tanstack/react-virtual` preventing main thread lag. | S2 | AC-06.2.4 |
| TC-06-07 | Swap to Mobile Dropdown Point | Positive | High | Screen `<768px` | 1. Decrease viewport boundaries below 768px. | Component transforms into a `zIndex: 50` `position: fixed` absolute overlay drawer in ≤ 100ms. | S2 | AC-06.3.1 |
| TC-06-08 | Mobile Auto-Hide Interaction | Positive | Med | Touch/Click Event | 1. Set viewport <768px.<br>2. Expand Drawer.<br>3. Select an underlying Object item. | Drawer automatically closes in ≤ 200ms revealing the full-width folder grid. | S3 | AC-06.3.2 |
| TC-06-09 | Transparent Mobile Grid Interaction | Negative | Med | Background click | 1. Set viewport <768px.<br>2. Ensure drawer is closed.<br>3. Click content under the drawer area. | Click passes through to grid items; there is no invisible overlay blocking interaction. | S3 | AC-06.3.3 |
| TC-06-10 | Mid-session Window rotate | Edge | Low | `Mobile context swap` | 1. Start <768px (Drawer open).<br>2. Resize OS window past >768px breakpoint. | Component converts back from 'drawer' to 'panel' without requiring a page reload. | S3 | AC-06.3.4 |
| TC-06-11 | [Implied] Drawer Backdrop Dismissal | Implied | Med | `Drawer layout open` | 1. Force <768px view.<br>2. Open Drawer.<br>3. Click on the dark layout backdrop natively outside drawer list bounds. | Drawer dismisses upon backdrop click matching web standards. | S3 | N/A |

## D. Missing / Implied Test Areas

- **Category State Flashing**: Do the toggle chevrons visually flash open/closed during the React Query initial population loop? Ensure state is stable until query resolves.

## E. Open Questions / Gaps

- **Debounce Resizer Rate**: Ensure React `react-resizable-panels` leverages debouncing before calling `localStorage.setItem` to avoid throttling API limits.

## F. Automation Candidates

- **TC-06-01**: Playwright or Cypress visual testing asserting the width pixel value matches the localized cache upon page restart.
- **TC-06-06**: Vitest for the `VirtualizedObjectList` component ensuring only visible items exist in DOM tree when fed an array of 500 elements.

## G. Test Environment Setup

- **Preconditions**: Dev Database populated with >50 objects linked to the `GameSchema` to render multiple category sections.

## H. Cross-Epic E2E Scenarios

- **E2E-06-01 (Massive Object Scaling)**: Combines Mod List (Epic 07) data populated with massive mod imports. The user expands categories holding hundreds of mods simultaneously. Virtualization must engage (Epic 06), ensuring the UI retains a smooth frame rate (≤16ms) entirely avoiding DOM bloat freeze.
