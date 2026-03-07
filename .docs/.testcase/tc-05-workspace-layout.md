# Test Cases: Workspace Layout & Navigation (req-05)

## A. Requirement Summary

- **Feature Goal**: Build the primary layout framework featuring a fluid 3-panel horizontal resize workspace (ObjectList, Explorer, Preview), top-bar primary navigation, and actionable launching UI elements, preventing jarring state losses across user usage.
- **User Stories**:
 - US-05.1: 3-Panel Resizable Workspace
 - US-05.2: Top Bar Navigation
 - US-05.3: Actionable Launch Bar
- **Success Criteria**:
 - Drag resize visuals operate at 60fps (≤16ms).
 - Width properties persisted`localStorage` on stop ≤200ms. Restored ≤50ms.
 - Inter-App routing navigates without remounting generic DOM chunks (≤100ms).
 - Game switches populate via invalidating specific data models ≤200ms.
 - Physical minimum screen bounds enforce limits. Width bounds respected absolutely.
- **Main Risks**: Visual lag during continuous resizing frames, React remounting issues losing input logic on state layout shifts, background task interruptions natively unmanaged regarding launching logic sequences.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-05-workspace-layout.md`

- AC-05.1.1, AC-05.1.2 → TC-05-01
- AC-05.1.3 → TC-05-02
- AC-05.1.4 → TC-05-03
- AC-05.2.1 → TC-05-04
- AC-05.2.2 → TC-05-05
- AC-05.2.3 → TC-05-06
- AC-05.2.4 → TC-05-07
- AC-05.3.1, AC-05.3.3 → TC-05-08
- AC-05.3.2 → TC-05-09
- AC-05.3.4 → TC-05-10
- AC-05.3.5 → TC-05-11

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| -------- | ----------------------------------- | -------- | -------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- |
| TC-05-01 | Horizontal Dividers functionality | Positive | High |`Mouse interaction` | 1. Launch App.<br>2. Click, drag Divider A (ObjectList/Explorer) left/right.<br>3. Drag Divider B (Explorer/Preview).<br>4. Reload Application. | DOM renders fluidly alongside scaling. Changes reflect on hard refresh exactly as prior left (within fractions stored cache mappings). | S3 | AC-05.1.1, AC-05.1.2 |
| TC-05-02 | Minimum Width Enforcement | Negative | Med |`OS cursor drag` | 1. Grab ObjectList divider.<br>2. Crush ObjectList left heavily past 200px limit.<br>3. Crush explorer right deeply. | Div boundary halts exactly at absolute pixel threshold (ex. ObjectList 180px). Refuses internal clipping behaviors. | S3 | AC-05.1.3 |
| TC-05-03 | Hard window squeeze format | Edge | Med |`Screen resizing OS` | 1. Decrease application global outer window bounding width under 1024px heavily via OS window corners.<br>2. Observe flex ratios. | Panels proportionally compress maintaining absolute ratio down to individual flex properties. Core layout does not detach UI components. | S4 | AC-05.1.4 |
| TC-05-04 | Popover Routing menu | Positive | High |`App interaction` | 1. Click user top bar icon drop-down.<br>2. Select internal section (ex. Collections).<br>3. Observe rendering context transition. | Instantly shifts active route without flash loads retaining surrounding structures, and UI indicator explicitly represents active state. | S3 | AC-05.2.1 |
| TC-05-05 | Switch Active System Selector | Positive | High |`DB 2 Games` | 1. Add Game A and Game B.<br>2. Open Mod Manager.<br>3. Adjust game selector top element.<br>4. Observe file trees matching targets. | Data context mutates. Folder polling invalidates, Grid redraws accurately for target entity. All responses <200ms mapped. | S2 | AC-05.2.2 |
| TC-05-06 | Redundant Location Routing | Negative | Low |`Dashboard Route` | 1. Route to Dashboard.<br>2. Open menu, click Dashboard repeatedly inside UI map 5+ times aggressively.<br>3. Check developer console / traces. | Navigation is suppressed natively logic returns`no-op`. Doesn't re-trigger React re-mount cycle logic or network hooks. | S4 | AC-05.2.3 |
| TC-05-07 | Shortcut collision bindings | Edge | Med |`Alt+key` | 1. Open Settings Modal Route.<br>2. Execute OS/Browser`Alt+1` navigation keystrokes implicitly or`Back` events forcefully. | Navigation does not jump blindly out underneath top rendering Z-Index block instance rendering form states gracefully maintaining active context. | S3 | AC-05.2.4 |
| TC-05-08 | Play interaction triggers Exec | Positive | High |`Clean Game` | 1. Configure standard functional target.<br>2. Ensure "Play" is glowing green.<br>3. Click Play exclusively. | Mod loader + wrapper executable routine boots. App wrapper automatically closes conditionally config boolean set. | S1 | AC-05.3.1, AC-05.3.3 |
| TC-05-09 | Contextual conflict badge warning | Positive | High |`DB conflicts data` | 1. Import colliding mod resources actively generating Conflict DB rows.<br>2. Open view targeting workspace UI. | Badge directly updates displaying amber indicator + precise error counts. Native interaction links specifically to detailed log sequence dashboard route. | S2 | AC-05.3.2 |
| TC-05-10 | Disabled launcher interaction | Negative | Med |`Blank Config` | 1. Attempt to interact with Launch button natively whilst configuration holds zero linked loader endpoints. | Action is physically disabled natively via property styling indicating visual tooltip response gracefully blocking routine. | S3 | AC-05.3.4 |
| TC-05-11 | Play button vs Background Mod | Edge | High |`OperationLock heavy` | 1. Select 50 heavy mods.<br>2. Execute toggle explicitly acquiring mutex lock.<br>3. Instantly jump hit Play Button. | Launch intercepts requesting confirmation regarding potentially ruining atomic task sequences. Safe bounds established preventing UI thread blocking. | S1 | AC-05.3.5 |
| TC-05-12 | [Implied] Local Storage Limit Clear | Implied | Low |`localStorage corrupted` | 1. Open Chromium DevTools.<br>2. Wipe`localStorage` arrays targeting split-pane states.<br>3. Reload App. | Interface defaults comfortably to default preset JSON standard configuration visually rather than breaking violently. | S4 | N/A |

## D. Missing / Implied Test Areas

- **Preview ObjectList auto-opening behaviors**: Often when checking a mod physically inside Grid logic the Preview pane interacts/opens. Does the pane resizing remember closed states, or purely`minWidth` bound metrics? (Can it be hidden directly without`0px` flex width?)
- **Route persistence on Game Swap**: If viewing`Settings`, and swapping currently active Game via navigation. Does it navigate you back to`Mods Manager`/`Dashboard`, or does the UI persist to wait for the user action routing event.

## E. Open Questions / Gaps

- "Panel proportions update fluidly at >= 60fps" - Depending on inner content logic rendering (e.g., highly populated list), rapid reflows might inherently cause lag beyond React strict parameters without virtualization limits applied consistently.

## F. Automation Candidates

- **TC-05-02 (Flex Width Resize Stop)**: Playwright assertions dragging target boundaries manually mapping position X properties to physical component metrics ensuring 0-delta change post validation logic maximum limit.
- **TC-05-06 (No-Op Redux Bounce)**: Cypress routing mock logic checks counting total router cycle events tracking DOM stability parameters across inputs.

## G. Test Environment Setup

- **Preconditions**: A minimum of Two distinct`Game` environments populated holding >30 Mods respectively ensuring dynamic caching drops testing active workspace transitions.
- **Screen Simulation**: Virtualized viewport configurations ranging 800px up safely simulating physical structural flex limits.

## H. Cross-Epic E2E Scenarios

- **E2E-05-01 (Workspace Routing Flow)**: Starting Dashboard (Epic 33), verifying 3-panel split exclusively via Settings (Epic 04) navigation, toggling target contexts (Epic 02), triggering Play button explicitly executing.
