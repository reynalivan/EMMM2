# Test Cases: Dashboard & Analytics (Epic 33)

## A. Requirement Summary

- **Feature Goal**: Provide a lightweight read-only UI homepage showing high-level telemetry (mod counts, enablement counts, disk size usage, duplicate waste) and parsed 3DMigoto Keybinding`d3dx.ini` settings for reference.
- **User Roles**: Application User.
- **Acceptance Criteria**:
 -`get_dashboard_stats` returns <200ms utilizing indexed SQLite`SUM/COUNT` arrays.
 - Active Keybindings mapping renders from the raw game folder's`d3dx.ini` limits <100ms.
 - Interactive Recharts show categories.
 - Graceful rendering empty fallbacks preventing numeric panics (NaNs or crashing loaders).
 - Phase 5: Charts (PieChart/BarChart with ARIA accessibility).
 - Phase 5: Activity Hub (Recently Added list).
 - Phase 5: Key Mapping Widget parsed from INI.
 - Phase 5: Cache staleTime 30s configs for rapid navigation.
 - Phase 5: Duplicate waste banner visually alerts to space savings.
- **Success Criteria**: Dashboard loads instantly. No heavy I/O blocking. Data accurately reflects DB state.
- **Main Risks**: Calculating total size triggering full directory walks instead of using DB caches, causing massive UI freezes on startup.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-33-dashboard.md`

| Acceptance Criteria | Covered by TC IDs |
| :------------------------------------- | :---------------- |
| AC-33.1.1 (Aggregate Multi-game Stats) | TC-33-001 |
| AC-33.1.2 (Duplicate Space Banner) | TC-33-002 |
| AC-33.1.3 (DaisyUI Theme Match) | TC-33-003 |
| AC-33.1.4 (Inaccessible DB Disk Fail) | TC-33-004 |
| AC-33.1.5 (Total Empty Welcome) | TC-33-005 |
| Phase 5: Recharts (Pie/Bar with ARIA) | TC-33-006 |
| Rechart Edge Cases | TC-33-007 |
| Phase 5: Activity Hub (Recent) | TC-33-008 |
| AC-33.3.2 (Quick Play Launcher) | TC-33-009 |
| AC-33.3.4 (Safe Mode Filter Recent) | TC-33-010 |
| Phase 5: Key Mapping Widget | TC-33-011 |
| Missing/Corrupt INI Fallback | TC-33-012 |
| Phase 5: Cache staleTime 30s | TC-33-013 |
| DB Lock Suspend Logic | TC-33-014 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :------------------------------- | :------- | :------- | :--------------- | :-------------------------------------------------------------------------------------------- | :-------------------------------- | :------------------------------------------------------------------------------------------------------------ | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :----------- |
| TC-33-001 | Global Stat Aggregation | Positive | High | S1 | DB populated with 5000+ objects across all games. | SQLite test DB | 1. Open Dashboard UI.<br>2. Inspect Total/Enabled count numbers. | Instantly calculates SQL`COUNT/SUM`. Renders`<200ms` skipping file I/O scans. Totals accurately match DB counts. | AC-33.1.1 |
| TC-33-002 | Wasted Duplicate UI Alert Banner | Positive | Medium | S2 | Dedup scanner was previously run and found duplicates. |`duplicate_waste_bytes = 15GB`. | 1. Load Dashboard. | System detects nonzero dupe size in DB. Prominently displays "⚠ 15GB wasted on duplicates" (Phase 5). Clicking banner routes to Dedup Scanner. | Phase 5 |
| TC-33-003 | Component Styling Tokens | Positive | Low | S3 | Native OS set to Dark Mode. EMMM2 set to 'System' theme. | Global Theme: Dark system. | 1. View Dashboard components. | StatCards pull correct DaisyUI background and text tokens directly, matching the dark theme aesthetically. | AC-33.1.3 |
| TC-33-004 | Disk Size Timeout Degrade | Edge | Medium | S2 | DB file`sqlite.db` is temporarily locked by an external SQLite browser during a write query. | Locked DB Handle | 1. Force the database lock.<br>2. Trigger Dashboard calculation. | Rust Backend times out.`StatCard` explicitly reads "Data unavailable" gracefully instead of looping infinitely. UI does not crash. | AC-33.1.4 |
| TC-33-005 | Zero Initial State Render | Edge | High | S1 | Fresh installation. Empty`games` and`folders` DB. | Empty SQLite tables. | 1. Open Dashboard layout. | Shows specific generic "Welcome! Add your first game" page hero. Never crashes via mathematical`NaN` divides in chart logic or raw blank white screens. | AC-33.1.5 |
| TC-33-006 | Phase 5: Interactive Recharts | Positive | Medium | S2 | DB initialized with 6 categories across 3 Games. Screen reader active. | Populated DB | 1. View visual plots.<br>2. Hover over chart slices.<br>3. Read with screen reader. | Pie Chart slices object data. Tooltips show percentages. Bar chart stacks per-game. ARIA labels announce chart metrics for accessibility (Phase 5). | Phase 5 |
| TC-33-007 | Single / Null Recharts Base | Edge | Low | S3 | DB holds exactly 1 generic category total. Only 1 game exists. | Minimal DB. | 1. View charts. | Renders intact full pie circle. Doesn't crash on singular data points. | Edge Cases |
| TC-33-008 | Phase 5: Activity Hub List | Positive | Medium | S2 | DB contains 20 mods sorted by`date_added`. | Sequential DB inserts. | 1. Validate "Recently Added" section on Dashboard. | Fetches exactly`date_added DESC LIMIT 5`. Renders appending localized Thumbnail badges (Phase 5). | Phase 5 |
| TC-33-009 | Quick Play Execution | Positive | High | S1 | Valid`last_played_game_id` exists. Launch settings are configured. | Game Exe path valid. | 1. Click "Play" button on Dashboard. | Dispatches exact IPC command`launch_game()`. Game initializes. Button enters busy state during launch. | AC-33.3.2 |
| TC-33-010 | Safe Mode Privacy Filter | Positive | High | S1 | 2 of the 5 newest mods are flagged`is_safe=false`. Safe Mode is currently ON globally. |`is_safe=0` targets. SafeMode=ON. | 1. Ensure global Safe Mode Lock is ON.<br>2. Refresh Dashboard. | "Recently Added" filter trims items omitting`is_safe=false` from the feed visually, protecting contents immediately. Total counts subtract hidden elements. | AC-33.3.4 |
| TC-33-011 | Phase 5: Key Mapping Widget | Positive | High | S1 |`d3dx.ini` is present in the game's executable path mapping specific keys. | Valid`d3dx.ini` | 1. View Keybinding component on the Dashboard. | Component retrieves mapped inputs`[Key*]`, decoding values and formatting "Action -> Key" lists in the UI for quick reference (Phase 5). | Phase 5 |
| TC-33-012 | Game INI Missing Degrade | Negative | Medium | S2 |`d3dx.ini` is deleted physically from the game directory. | No config file. | 1. View Keybinding component. | Detects missing file. Renders basic text fallback "Keybindings not found" bypassing unhandled Rust`fs::read` exceptions entirely. | Missing INI |
| TC-33-013 | Phase 5: Cache staleTime 30s | Edge | Low | S3 | User relies on TanStack Query cache. | Action Timeline | 1. Open Dashboard.<br>2. Go to Explorer.<br>3. Add a Mod.<br>4. Return to Dashboard immediately (within 30s). | Returning to the dashboard before 30s loads instantly from memory. The 'Add a Mod' action fired`invalidateQueries`, bypassing the 30s limit to fetch the new accurate counts instantly. | Phase 5 |
| TC-33-014 | DB SQLite Lock Suspension | Negative | High | S1 | A background Rust thread deliberately locks the DB exclusively for a heavy write operation. | Heavy IPC background task. | 1. Attempt to interact with Dashboard while lock is held. | Frontend catches the timeout error, falling back to a shimmer skeleton rendering text "Refreshing data..." ensuring the UI never hard-freezes. | DB Interlock |

## D. Missing / Implied Test Areas

- **[Implied] Safe Mode Analytics**: Checking if global mathematical integer logic removes total enabled/disabled combinations exactly from standard UI queries when shielding. Yes, Dual Guard handles this.
- **[Implied] Resize Observer**: Are the charts responsive fitting containers resolving layout shifts during arbitrary scaling resizes.

## E. Open Questions / Gaps

- No specific questions. The Phase 5 UI requirements are fully mapped.

## F. Automation Candidates

- **TC-33-001 & TC-33-008**: Rust Backend SQLite testing validating explicitly mathematical`COUNT/SUM` SQL functions pull explicit correct aggregates.
- **TC-33-012**: File system mocking passing nil directories extracting precise string Error payload captures to ensure graceful degradation.

## G. Test Environment Setup

- **Database Condition**: Ensure`folders` table has at least 5000 records spread across 3 unique`game_id` variables to simulate a heavy user.
- **Duplicate Config**: Write a test entry directly to SQLite`duplicate_reports` with total integer summing to exactly 15GB to trigger the warning banner.
- **Safe Mode**: Tag 2 recently added items explicitly as`is_safe=0`. Target DB row with Safe Mode toggled`On`.
- **Game INI**: Create a dummy`d3dx.ini` within the active mocked game directory containing`[Key1]` mapping`key=F6`.

## H. Cross-Epic E2E Scenarios

- **E2E-33-001 (Dashboard Analytics to Import Pipeline)**: The user opens the App with Safe Mode enabled (Epic 30) observing the Recharts (Epic 33). The User drops a 5GB ZIP archive onto the window to trigger Mod Import (Epic 23). While import runs in the background, the UI is still responsive due to the indexed DB queries. Once the import finishes, it fires the TanStack Query invalidation (bypassing the 30s stale time). The Dashboard instantly reflects the newly imported mod count visually, the charts re-render slices, and the mod appears in the "Recently Added" Activity Hub list, complete with its extracted thumbnail.
