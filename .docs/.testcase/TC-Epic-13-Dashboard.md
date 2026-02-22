# Test Case Scenarios: Epic 13 - Global Dashboard & Executive Insights

**Objective:** Validate aggregated statistics accuracy, `Recharts` rendering, Safe Mode query filtering, cross-game totals, skeleton shimmer loading, and SQL index verification.

**Ref:** [epic13-dashboard.md](file:///e:/Dev/EMMM2NEW/.docs/epic13-dashboard.md) | TRD §2.2

---

## 1. Functional Test Cases (Positive)

### US-13.1: Global Overview Tiles

| ID             | Title                 | Pre-Condition              | Steps              | Expected Result                                                                      | Post-Condition | Priority |
| :------------- | :-------------------- | :------------------------- | :----------------- | :----------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-13.1-01** | **Stats Accuracy**    | - 100 mods across 2 games. | 1. View Dashboard. | - Total: 100.<br>- Per-game breakdown correct.<br>- Numbers match `SELECT COUNT(*)`. | Accurate.      | High     |
| **TC-13.1-02** | **Cross-Game Totals** | - 4 games configured.      | 1. View Dashboard. | - Totals correctly sum across all games.<br>- Storage total aggregated.              | Correct.       | High     |

### US-13.2: Distribution Charts

| ID             | Title                    | Pre-Condition                      | Steps                       | Expected Result                                                         | Post-Condition | Priority |
| :------------- | :----------------------- | :--------------------------------- | :-------------------------- | :---------------------------------------------------------------------- | :------------- | :------- |
| **TC-13.2-01** | **Pie Chart Render**     | - Multiple object types with data. | 1. View distribution chart. | - `Recharts` Pie Chart renders.<br>- Segments > 0.<br>- Legend visible. | Rendered.      | High     |
| **TC-13.2-02** | **Bar Chart (Per Game)** | - Mods per game data.              | 1. View charts.             | - Bar chart shows per-game mod counts.<br>- Tooltips on hover.          | Rendered.      | Medium   |

### US-13.3: Quick Actions & Recent Activity

| ID             | Title                  | Pre-Condition                   | Steps           | Expected Result                                                     | Post-Condition | Priority |
| :------------- | :--------------------- | :------------------------------ | :-------------- | :------------------------------------------------------------------ | :------------- | :------- |
| **TC-13.3-01** | **Last Played Launch** | - "Last Played: Genshin" shown. | 1. Click tile.  | - Launcher executes (reuses E10 service).<br>- Same as Play button. | Launched.      | Medium   |
| **TC-13.3-02** | **Recently Added**     | - 5 recent mod imports.         | 1. View widget. | - Top 5 recent mods shown.<br>- With thumbnail and date.            | Displayed.     | Medium   |

### US-13.4: Safe Mode Dashboard Filter

| ID             | Title            | Pre-Condition   | Steps              | Expected Result                                                                                       | Post-Condition | Priority |
| :------------- | :--------------- | :-------------- | :----------------- | :---------------------------------------------------------------------------------------------------- | :------------- | :------- |
| **TC-13.4-01** | **Privacy Sync** | - Safe Mode ON. | 1. View Dashboard. | - Stats exclude NSFW mods (`is_safe = 1`).<br>- "Recently Added" hides NSFW.<br>- Charts filter NSFW. | Filtered.      | High     |

---

## 2. Negative Test Cases (Error Handling)

### US-13.1: Data Errors

| ID             | Title                | Pre-Condition    | Steps                     | Expected Result                                                              | Post-Condition | Priority |
| :------------- | :------------------- | :--------------- | :------------------------ | :--------------------------------------------------------------------------- | :------------- | :------- |
| **NC-13.1-01** | **DB Query Error**   | - Corrupt DB.    | 1. View Dashboard.        | - Show empty state or "DB Error".<br>- No crash.<br>- Retry button shown.    | Handled.       | High     |
| **NC-13.1-02** | **Division by Zero** | - 0 mods total.  | 1. View percentage chart. | - Handle 0 gracefully: show "0%" not NaN.<br>- Empty chart placeholder.      | Safe.          | Medium   |
| **NC-13.1-03** | **Loading State**    | - Slow DB query. | 1. Open Dashboard.        | - Skeleton shimmer shown while loading.<br>- Target: < 200ms to first paint. | Shimmer.       | Medium   |

### US-13.2: Chart Errors

| ID             | Title                       | Pre-Condition          | Steps              | Expected Result                                                             | Post-Condition | Priority |
| :------------- | :-------------------------- | :--------------------- | :----------------- | :-------------------------------------------------------------------------- | :------------- | :------- |
| **NC-13.2-01** | **Zero Data Fresh Install** | - No games configured. | 1. View Dashboard. | - "Welcome! Add your first Game" empty state.<br>- No NaN, no empty charts. | Welcome.       | High     |

---

## 3. Edge Cases & Stability

| ID           | Title                     | Simulation Step                                               | Expected Handling                                            | Priority |
| :----------- | :------------------------ | :------------------------------------------------------------ | :----------------------------------------------------------- | :------- |
| **EC-13.01** | **Negative Size Anomaly** | 1. DB row `size_bytes < 0` (corruption).                      | - Clamp to 0.<br>- Log data anomaly.                         | Low      |
| **EC-13.02** | **Overflow (Petabyte)**   | 1. Huge `size_bytes` value.                                   | - Format string: "PB" correctly.<br>- No integer overflow.   | Low      |
| **EC-13.03** | **Massive History**       | 1. 1M recent entries.                                         | - Query uses `LIMIT 5`.<br>- Performance unaffected.         | High     |
| **EC-13.04** | **Stale Cache Refresh**   | 1. TanStack Query `staleTime: 30_000ms`.<br>2. Click refresh. | - Manual refresh invalidates cache.<br>- Fresh data fetched. | Medium   |
| **EC-13.05** | **Rapid Game Add/Remove** | 1. Add game → View dashboard → Remove game.                   | - Dashboard updates in real-time.<br>- No stale data shown.  | Medium   |

---

## 4. Technical Metrics

| ID           | Metric             | Threshold   | Method                                                      |
| :----------- | :----------------- | :---------- | :---------------------------------------------------------- |
| **TM-13.01** | **Query Time**     | **< 50ms**  | All aggregation queries. Verified via `EXPLAIN QUERY PLAN`. |
| **TM-13.02** | **Memory Payload** | **< 5KB**   | Dashboard JSON response size.                               |
| **TM-13.03** | **First Paint**    | **< 200ms** | Skeleton shimmer to data rendered.                          |

---

## 5. Data Integrity

| ID           | Object                   | Logic                                                                                     |
| :----------- | :----------------------- | :---------------------------------------------------------------------------------------- |
| **DI-13.01** | **Read Only**            | Dashboard queries MUST be `SELECT` only. No side effects.                                 |
| **DI-13.02** | **Index Scan**           | All queries use indexed columns. Verified: `EXPLAIN QUERY PLAN` shows no full table scan. |
| **DI-13.03** | **Safe Mode SQL**        | When `safeMode = true`, all queries append `AND is_safe = 1`.                             |
| **DI-13.04** | **Recharts Integration** | Charts use `Recharts` library (per TRD). No other charting libs.                          |
