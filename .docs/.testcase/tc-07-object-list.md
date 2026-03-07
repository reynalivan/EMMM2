# Test Cases: Object List (req-07)

## A. Requirement Summary

- **Feature Goal**: A high-performance, virtualized list for objectlist objects that handles real-time mod count updates, batch operations, and drag-and-drop categorization via folders.
- **User Stories**:
 - US-07.1: Virtualized Rendering
 - US-07.2: Object Selection & Navigation
 - US-07.3: Dynamic Enabled Counts
 - US-07.4: Drag-and-Drop Mod Re-Categorization
- **Success Criteria**:
 - Virtualized rendering handles 1000+ items hitting 60fps.
 - Active selection invalidates Grid queries (≤200ms).
 - Toggling mod updates objectlist counts optimistically (≤50ms).
 - Bulk actions batch count updates automatically reducing render ticks.
 - DnD visually drops target highlighting (≤50ms) and rewrites mapping (≤500ms).
- **Main Risks**: Mutating`selectedObjectId` without cleaning cache causing ghost state loops on Delete/Cross-Game events. Unbatched updates destroying React profiler times. Loss of drag coordinates when mapping over virtualized boundary indexes.
## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-07-object-list.md`

- AC-07.1.1, AC-07.1.2 → TC-07-01
- AC-07.1.3 → TC-07-02
- AC-07.1.4 → TC-07-03
- AC-07.2.1, AC-07.2.2 → TC-07-04
- AC-07.2.3 → TC-07-05
- AC-07.2.4 → TC-07-06
- AC-07.3.1, AC-07.3.2 → TC-07-07
- AC-07.3.3 → TC-07-08
- AC-07.3.4 → TC-07-09
- AC-07.4.1, AC-07.4.2, AC-07.4.3 → TC-07-10
- AC-07.4.4 → TC-07-11
- AC-07.4.5 → TC-07-12

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | --------------------------------- | -------- | -------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------- |
| TC-07-01 | Virtualized massive list render | Positive | High |`Dense DB` (1000+ objects) | 1. Launch App rendering 1000 objects.<br>2. Rapidly scroll mouse wheel bottom-to-top constantly for 3 seconds.<br>3. Profile DOM count. | DOM only mutates ~30 node containers. Screen doesn't suffer blank white delays >16ms consistently maintaining 60fps. | S2 | AC-07.1.1, AC-07.1.2 |
| TC-07-02 | Resource constraint degradation | Negative | Low |`Memory choked OS` | 1. Simulate heavy resource starvation externally.<br>2. Perform extremely fast scroll input via API or simulated wheel events heavily. | System avoids lockup. FPS may drop slightly but mouse interactions/clicks aren't permanently swallowed blocking UI threads. | S4 | AC-07.1.3 |
| TC-07-03 | Dynamic bound resize calculations | Edge | Med |`Application Resize` | 1. Open Object List.<br>2. Rapidly adjust window height down 50% via OS grabbing.<br>3. Stop and observe viewport. | Virtualization calculates new pixel constraints under 100ms. Blank blocks don't get 'stuck' unrendered forcing layout breaks. | S3 | AC-07.1.4 |
| TC-07-04 | Valid Object Selection | Positive | High |`Valid Object row` | 1. Wait for ObjectList loads.<br>2. Click target object row.<br>3. Observe Main Grid. | Zustand ID tracks. Highlight maps. Grid area invalidates previous load mapping replacing it (≤200ms). | S2 | AC-07.2.1, AC-07.2.2 |
| TC-07-05 | Object background deletion | Negative | High |`Orphaned Row selected via 1-sec cache` | 1. Select object.<br>2. Delete the physical folder for the object externally.<br>3. Attempt to interact with it natively in the list. | Soft failure. The selection defaults away securely without throwing stack trace/crash toasts bypassing stale caches. | S2 | AC-07.2.3 |
| TC-07-06 | Cross-game selection clearing | Edge | Med |`Context Shift` | 1. Select object in Game A.<br>2. Trigger top-bar Active Game Switch to Game B.<br>3. Check Active Object queries. | ID forcibly cleared to prevent React Query from asking for`Game B Target` +`Game A ObjectId` mix. | S2 | AC-07.2.4 |
| TC-07-07 | Optimistic active badges | Positive | High |`Active Folders` | 1. Navigate to Grid housing multiple mods.<br>2. Click 'Enable' specifically on a Mod.<br>3. Observe ObjectList counts immediately. | ObjectList immediately responds visually mutating`Enabled/Total` tag prior to API HTTP-equivalent resolve (≤50ms). | S3 | AC-07.3.1, AC-07.3.2 |
| TC-07-08 | 0/X Dim styling | Negative | Low |`Object List (1 Disabled Mod)` | 1. Disable all mods in a category/object.<br>2. Ensure object counts show`0/Total`.<br>3. Verify CSS styling manually. | UI text or badge dims strictly confirming visually 'Inactive' state representation. | S4 | AC-07.3.3 |
| TC-07-09 | Batched bulk updates | Edge | Med |`100 Mod entries` | 1. Bulk select 100 mods.<br>2. Activate standard 'Toggle Selected' hook simultaneously.<br>3. Profile rendering ticks. | ObjectList does NOT fire 100 individual re-render loops. Bundles to a singular render context modifying specific state indexes. | S2 | AC-07.3.4 |
| TC-07-10 | Seamless Drag and Drop Flow | Positive | High |`Target Object in ObjectList` | 1. Drag Folder from Grid.<br>2. Hover over ObjectList Object component.<br>3. Drop payload. | Highlight reacts ≤50ms. Disk IO physically fires`move_mod`. Caches both (Src + Target) violently refresh triggering Grid list sync (≤500ms). | S1 | AC-07.4.1, AC-07.4.2, AC-07.4.3 |
| TC-07-11 | DnD mapping collision | Negative | High |`Conflict identical named Folders` | 1. Drag Folder.<br>2. Move into category already holding duplicate raw dir name physically.<br>3. Drop. | Process aborts securely throwing logical`CollisionError` mapping to Conflict Resolution workflow. Original directory persists safe. | S1 | AC-07.4.4 |
| TC-07-12 | DnD crossing virtual borders | Edge | Med |`List index target beyond initial paint` | 1. Hover target list edge inducing auto-scroll.<br>2. Drop on created visual block.<br>3. Check index destination. | Coordinate maps structurally to exact Virtual ID hook references, avoiding 'undefined' drops dropping into the void. | S2 | AC-07.4.5 |
| TC-07-13 | [Implied] Safe mode zeroing | Implied | Med |`Object strictly housing NSFW mods` | 1. Mark target safe.<br>2. Enable secure privacy mode boolean exclusively via Settings.<br>3. Inspect objectlist badge counters. | Total badge logic securely masks explicitly (e.g. outputs simple`0/0` counts, or filtering natively). Prevents implicit total knowledge leak. | S3 | N/A |

## D. Missing / Implied Test Areas

- **List auto-scrolling during drag**: Explicit test needed to verify that dragging a mod to the bottom or top of the objectlist actually begins auto-scrolling the viewport if the item is nested deep down.

## E. Open Questions / Gaps

- **Resolved**: Dragging visually uses the native OS-level drag overlay because EMMM2 uses Tauri's physical file dragging (`tauri://drag-enter`), abandoning the React-dom limited`dnd-kit`. Auto-scrolling on container edges is natively handled via`useDragAutoScroll` hook explicitly for massive virtualized lists.

## F. Automation Candidates

- **TC-07-04 (Selection Linkages)**: React Testing Library standard pipeline. Asserting`onClick` fires corresponding callback parameters targeting Zustand.
- **TC-07-07 (Immediate Optimism)**: E2E test verifying query client`.setQueryData` reacts of the mock backend execution resolver timeout.

## G. Test Environment Setup

- **Preconditions**: SQLite DB mapped storing`>1000` objects specifically testing DOM Virtualization parameters without relying simulating constraints.
- **Test Locations**: DND folder boundaries generated containing identical duplicates verifying collision logic.

## H. Cross-Epic E2E Scenarios

- **E2E-07-01 (DnD Optimistic Sync)**: User drags Mod from Explorer (Epic 15) dropping onto ObjectList (TC-07-10), triggering a physical file move. This validates DB sync (Epic 27) by confirming ObjectList badges and the Grid update appropriately to reflect the new state.
