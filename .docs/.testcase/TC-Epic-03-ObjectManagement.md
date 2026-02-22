# Test Case Scenarios: Epic 3 - Game Object Management

**Objective:** Validate sidebar navigation, dynamic categorization, filtering, virtualized list rendering, schema.json fallback, and Safe Mode filtering.

**Ref:** [epic3-game-object-manage.md](file:///e:/Dev/EMMM2NEW/.docs/epic3-game-object-manage.md) | TRD §2.1, §2.2

---

## 1. Functional Test Cases (Positive)

### US-3.1: Multi-Category Navigation

| ID            | Title               | Pre-Condition              | Steps                      | Expected Result                                                    | Post-Condition    | Priority |
| :------------ | :------------------ | :------------------------- | :------------------------- | :----------------------------------------------------------------- | :---------------- | :------- |
| **TC-3.1-01** | **Switch Game**     | - GIMI + WWMI configured.  | 1. Select WWMI in sidebar. | - Sidebar loads WWMI objects.<br>- Zustand `activeGameId` updated. | Context switched. | High     |
| **TC-3.1-02** | **Category Filter** | - Multiple object types.   | 1. Filter by "Character".  | - Only Character objects shown.<br>- Count badge updates.          | Filtered.         | High     |
| **TC-3.1-03** | **Alias Search**    | - "Raiden" has alias "Ei". | 1. Type "Ei" in search.    | - "Raiden Shogun" appears in results.<br>- Search latency < 50ms.  | Found.            | Medium   |

### US-3.3: Object CRUD

| ID            | Title                   | Pre-Condition                  | Steps                              | Expected Result                                                  | Post-Condition  | Priority |
| :------------ | :---------------------- | :----------------------------- | :--------------------------------- | :--------------------------------------------------------------- | :-------------- | :------- |
| **TC-3.3-01** | **Create Object**       | - Sidebar context active.      | 1. Click "Add".<br>2. Name "Eula". | - Folder created on disk.<br>- DB row inserted.                  | Object visible. | High     |
| **TC-3.3-02** | **Sync Metadata**       | - Object missing metadata.     | 1. Click "Sync" on object.         | - Metadata fetched (Cryo/Claymore).<br>- `schema.json` updated.  | JSON updated.   | High     |
| **TC-3.3-03** | **Drag & Drop Reorder** | - Multiple objects in sidebar. | 1. Drag "Eula" above "Raiden".     | - Order persisted to DB.<br>- UI reflects new order immediately. | Reordered.      | Medium   |

### US-3.5: Virtualized List

| ID            | Title                 | Pre-Condition         | Steps            | Expected Result                                                                  | Post-Condition | Priority |
| :------------ | :-------------------- | :-------------------- | :--------------- | :------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-3.5-01** | **500+ Items Render** | - 500 objects loaded. | 1. Open sidebar. | - TanStack Virtual active.<br>- Only ~20 DOM nodes rendered.<br>- Smooth scroll. | FPS > 50.      | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-3.3: CRUD Failures

| ID            | Title                | Pre-Condition                | Steps             | Expected Result                                                      | Post-Condition | Priority |
| :------------ | :------------------- | :--------------------------- | :---------------- | :------------------------------------------------------------------- | :------------- | :------- |
| **NC-3.3-01** | **Create Duplicate** | - "Eula" exists.             | 1. Create "Eula". | - Error: "Object already exists".                                    | Blocked.       | High     |
| **NC-3.3-02** | **Delete Non-Empty** | - "Eula" has mods.           | 1. Delete "Eula". | - Prompt: "Folder not empty. Delete all mods?".<br>- Cancel → Abort. | Safe.          | Medium   |
| **NC-3.3-03** | **Invalid Name**     | - Input: `COM1` or `../../`. | 1. Rename.        | - Error: "Invalid Name".<br>- Reserved names blocked.                | Blocked.       | High     |

### US-3.4: Filtering/Schema Failures

| ID            | Title                   | Pre-Condition                       | Steps              | Expected Result                                                                   | Post-Condition | Priority |
| :------------ | :---------------------- | :---------------------------------- | :----------------- | :-------------------------------------------------------------------------------- | :------------- | :------- |
| **NC-3.4-01** | **No Filter Results**   | - Filter "Dendro" (none exist).     | 1. Select filter.  | - Empty state: "No objects match filter".                                         | UI stable.     | Low      |
| **NC-3.4-02** | **Schema Load Failure** | - `schema.json` missing or corrupt. | 1. Switch to game. | - Fallback: load objects from DB/filesystem.<br>- Warning: "Using fallback data". | Functional.    | High     |

---

## 3. Edge Cases & Stability

| ID          | Title                        | Simulation Step                              | Expected Handling                                                                      | Priority |
| :---------- | :--------------------------- | :------------------------------------------- | :------------------------------------------------------------------------------------- | :------- |
| **EC-3.01** | **Rapid Game Switching**     | 1. Click Genshin → StarRail → ZZZ (100ms).   | - Cancel pending queries (TanStack Query).<br>- Load ONLY ZZZ.<br>- No race condition. | High     |
| **EC-3.02** | **External Deletion**        | 1. Delete "Eula" folder in Explorer.         | - Watchdog detects removal.<br>- UI removes instantly.<br>- Log info.                  | High     |
| **EC-3.03** | **Database Lock**            | 1. Lock DB externally.<br>2. Try Add Object. | - Error: "Database Busy".<br>- Retry 3x → Fail gracefully.                             | Medium   |
| **EC-3.04** | **Case Sensitivity**         | 1. "Raiden" exists.<br>2. Add "raiden".      | - Treated as duplicate (Windows).<br>- Creation blocked.                               | Medium   |
| **EC-3.05** | **Safe Mode Sidebar Filter** | 1. Safe Mode ON.<br>2. View sidebar.         | - Objects containing only NSFW mods not shown.<br>- Mod counts exclude NSFW.           | High     |

---

## 4. Technical Metrics

| ID          | Metric             | Threshold   | Method                                       |
| :---------- | :----------------- | :---------- | :------------------------------------------- |
| **TM-3.01** | **Render Latency** | **< 200ms** | Render 500 items sidebar (TanStack Virtual). |
| **TM-3.02** | **Search Lag**     | **< 50ms**  | Keypress to filtered list.                   |
| **TM-3.03** | **Game Switch**    | **< 300ms** | Click game → sidebar fully loaded.           |

---

## 5. Data Integrity

| ID          | Object              | Logic                                                                               |
| :---------- | :------------------ | :---------------------------------------------------------------------------------- |
| **DI-3.01** | **Cascades**        | Deleting Object row must cascade (or warn) about associated `mods` rows.            |
| **DI-3.02** | **Schema Fallback** | If `schema.json` missing, app must still function using DB/filesystem data.         |
| **DI-3.03** | **Zustand Sync**    | `activeGameId` in Zustand must always match the currently rendered sidebar context. |
