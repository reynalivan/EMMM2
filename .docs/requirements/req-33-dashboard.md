# Epic 33: Dashboard & Analytics

## 1. Executive Summary

- **Problem Statement**: After onboarding, users have no at-a-glance view of their modding setup — they must manually browse the objectlist to understand library composition, cross-game context is invisible, and there's no quick reference for 3DMigoto keybindings without Alt-Tabbing.
- **Proposed Solution**: A Dashboard page showing: (1) global and per-game aggregated statistics (total mods, enabled/disabled counts, total disk size, duplicate waste) via SQLite `COUNT`/`SUM` queries; (2) interactive `Recharts` charts (Pie: category distribution, Bar: mods per game); (3) a "Recently Added" feed of the 5 latest mods; (4) a "Quick Play" shortcut to the last-played game; (5) an Active Key Mapping widget showing current mod keybinds — all cached with React Query `staleTime: 30s` and a manual refresh button.
- **Success Criteria**:
  - `get_dashboard_stats` returns in ≤ 200ms for ≤ 5,000 mods (SQLite `COUNT`/`SUM`, indexed, no file I/O).
  - `get_active_keybindings` parses `d3dx.ini` and returns in ≤ 100ms.
  - All aggregation queries use `EXPLAIN QUERY PLAN`-verified index scans — no full table scans.
  - Dashboard JSON payload ≤ 5KB per response.
  - Dashboard renders fully within ≤ 500ms of navigation (first paint + data fetch).
  - Missing or unreadable `d3dx.ini` shows a graceful empty state — no crash.
  - Dashboard statistics auto-refresh after bulk ops via `staleTime` expiry or `invalidateQueries`.

---

## 2. User Experience & Functionality

### User Stories

#### US-33.1: View Global Statistics

As a user, I want to see a cross-game summary of my mod library, so that I know the total scope of my collection without checking each game individually.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                  |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-33.1.1 | ✅ Positive | Given the Dashboard is open, when it loads, then `StatCard` components display: total mods (all games), enabled/disabled distribution, total game count, total collections, and total disk size (GB/MB) — all respecting the active Safe Mode corridor. |
| AC-33.1.2 | ✅ Positive | Given `duplicate_waste_bytes > 0` (Epic 32 dedup scan ran), then an interactive banner "⚠ {size} wasted on duplicates — click to scan" appears on the Dashboard; clicking navigates to the Dedup Scanner                                                  |
| AC-33.1.3 | ✅ Positive | Given a dark mode theme, then all `StatCard` components use the design system's card tokens — consistent visual weight with the rest of the app                                                                                                           |
| AC-33.1.4 | ❌ Negative | Given the `mods_path` directory is inaccessible, then the disk-size `StatCard` shows "Size unavailable" — other stats (DB `COUNT/SUM`) still display correctly                                                                                            |
| AC-33.1.5 | ⚠️ Edge     | Given 0 mods are indexed (fresh install), then the Dashboard shows "Welcome! Add your first game to begin" empty state — no NaN, no zero-divide errors, no blank tiles                                                                                    |

---

#### US-33.2: Analytics Charts

As a mod collector, I want to see visual breakdowns of my mod distribution, so that I can understand which categories and games dominate my library.

| ID        | Type        | Criteria                                                                                                                                                                                                  |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-33.2.1 | ✅ Positive | Given `object_type` data is available, when the Dashboard loads, then an interactive `Recharts` Pie Chart renders dividing mods by `object_type` (Character / Weapon / UI / Other) with percentage labels |
| AC-33.2.2 | ✅ Positive | Given ≥ 2 games are configured, when the Dashboard loads, then a `Recharts` Bar Chart shows the mod count per game (e.g., Genshin: 200, HSR: 80, ZZZ: 45) — one bar per game, sorted descending           |
| AC-33.2.3 | ❌ Negative | Given only 1 `object_type` exists across all mods, then the Pie Chart renders as a single-segment circle with a note "Only one category present" — no chart rendering error                               |
| AC-33.2.4 | ⚠️ Edge     | Given `total_mods = 0` for a game (no mods), that game is excluded from the Bar Chart entirely — not shown as a 0-height bar                                                                              |

---

#### US-33.3: Activity Hub — Recently Added & Quick Play

As a busy user, I want quick access to recently added mods and the last game I played, so that I can jump right into action.

| ID        | Type        | Criteria                                                                                                                                                                       |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-33.3.1 | ✅ Positive | Given the Dashboard loads, then a "Recently Added" feed shows the 5 most recently indexed mods (by `indexed_at DESC`) with name, game name, object name (if any), and relative timestamp. |
| AC-33.3.2 | ✅ Positive | Given an active game, then a "Quick Play" button appears — clicking fires the same launch logic as the game switcher launch. |
| AC-33.3.3 | ❌ Negative | Given only 2 mods exist in the library, then the "Recently Added" feed shows only 2 cards — no empty/null placeholders. |
| AC-33.3.4 | ⚠️ Edge     | Given Safe Mode is ON, then the "Recently Added" feed filters out `is_safe = false` mods — NSFW entries do not appear in the feed. |

---

#### US-33.4: Active Key Mapping Widget

As a user, I want to see which keybinds my currently active mods define, so that I know which keys to press in-game without memorizing them.

| ID        | Type        | Criteria                                                                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-33.4.1 | ✅ Positive | Given the active game's `d3dx.ini` is found, when the Dashboard loads, then `KeybindingList` displays common `[Key*]` block shortcuts: `Reload`, `CycleMods`, `ShowFPS`, and any user-defined key sections (max 20 entries) |
| AC-33.4.2 | ✅ Positive | Given mod keybind metadata is stored (Epic 43 harvest), then an "Active Mod Keybinds" section shows character-scoped mappings like "Raiden Shogun: F6 → toggle aura" from enabled mods                                      |
| AC-33.4.3 | ❌ Negative | Given `d3dx.ini` does not exist or is unreadable, then the Keybindings section shows "Keybindings not found — ensure your 3DMigoto installation is configured" — no crash                                                   |
| AC-33.4.4 | ⚠️ Edge     | Given `d3dx.ini` contains a malformed `[Key*]` section (missing `key =` line), then that section is skipped; valid sections are still displayed                                                                             |

---

#### US-33.5: Cache & Refresh

As a system, I want dashboard data to be cached and refreshable, so that the UI stays fast during normal browsing but updates are visible after bulk operations.

| ID        | Type        | Criteria                                                                                                                                                                     |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-33.5.1 | ✅ Positive | Given Dashboard stats/distributions, they are cached with `staleTime: 30s` for rapid navigation access. |
| AC-33.5.2 | ✅ Positive | Given active keybindings, they are cached with `staleTime: 60s` due to the intensive filesystem scan required for .ini parsing. |
| AC-33.5.3 | ✅ Positive | Given a manual "Refresh" button, when clicked, then `invalidateQueries` fires and fresh data loads (skeleton shimmer visible during load). |
| AC-33.5.3 | ❌ Negative | Given a DB aggregation query fails (DB locked), then the dashboard shows a skeleton loader + "Refresh" button — no white screen or unhandled error boundary                  |

---

### Non-Goals

- No activity feed of individual mod toggle history (could be a future enhancement).
- Dashboard stats are read-only — no interactive enablement from the Dashboard.
- `d3dx.ini` keybinding display is reference-only — no in-app editing.
- No chart export or screenshot functionality.

---

## 3. Technical Specifications

### Architecture Overview

```
get_dashboard_stats() → DashboardStats:
  SELECT
    COUNT(*) as total_mods,
    SUM(CASE WHEN is_enabled = 1 THEN 1 ELSE 0 END) as enabled_mods,
    SUM(CASE WHEN is_enabled = 0 THEN 1 ELSE 0 END) as disabled_mods,
    SUM(size_bytes) as total_size_bytes,
    (SELECT SUM(wasted_size) FROM duplicates) as duplicate_waste_bytes
  FROM folders
  (all games — no game_id filter)

get_distribution_by_type() → Vec<CategoryDistribution { category, count }>:
  SELECT object_type as category, COUNT(*) as count
  FROM folders
  GROUP BY object_type
  ORDER BY count DESC

get_distribution_by_game() → Vec<GameDistribution { game_name, count }>:
  SELECT g.name as game_name, COUNT(f.id) as count
  FROM folders f JOIN games g ON f.game_id = g.id
  GROUP BY f.game_id
  ORDER BY count DESC

get_recent_mods(limit=5) → Vec<RecentMod { id, name, indexed_at, game_name, object_name }>:
  SELECT f.id, f.name, f.indexed_at, g.name as game_name, f.object_name
  FROM folders f JOIN games g ON f.game_id = g.id
  WHERE f.is_safe = ? 
  ORDER BY indexed_at DESC LIMIT 5

get_active_keybindings(game_id) → Vec<KeybindEntry>:
  1. Fetch enabled mods for game
  2. Walk each mod folder for .ini files
  3. Parse [Key*] sections → {mod_name, section_name, key, back}

Frontend:
  useDashboardStats() → staleTime: 30s
  useActiveKeybindings(gameId) → staleTime: 60s
  Quick Actions: Mods, Dedup, Collections, Settings, Discover, Downloads

Charts: Recharts PieChart + BarChart
  PieChart: data=distributionByType, colorPalette=designSystem
  BarChart: data=distributionByGame, XAxis=game_name, YAxis=count
```

### Integration Points

| Component        | Detail                                                                                                    |
| ---------------- | --------------------------------------------------------------------------------------------------------- |
| Charts           | `recharts` — `PieChart` for category distribution, `BarChart` for game distribution; all have ARIA labels |
| Stats Query      | Cross-game: no `game_id` filter — pure SQLite `COUNT`/`SUM` on all `folders`                              |
| Disk Size        | Pre-computed `size_bytes` in `folders` table (set during scan); fallback `du()` bounded 5s                |
| Keybindings      | Shared INI parser from Epic 18; supported by `DemoKeybindSpotlight` scene for visual emphasis             |
| Duplicate Banner | `duplicate_waste_bytes` from dedup cache (Epic 32) — if > 0 shows banner                                  |
| Quick Play       | `invoke('launch_game', { game_id: last_played_game_id })` — reuses Epic 01 launch logic                   |
| Demo Scenes      | Integrated `SmartDemoStrip`, `DemoKeybindSpotlight`, and `DemoTogglePreset` for landing/onboarding visuals|
| Cache            | React Query `staleTime: 30_000ms`; `invalidateQueries(['dashboardStats'])` after bulk ops                 |

### Security & Privacy

- **`d3dx.ini` path resolved via `game.install_path + /d3dx.ini`** — validated not to escape game directory.
- **Dashboard stats queries are read-only** — no DB mutations from this epic.
- **Safe Mode respected**: `get_recent_mods` and stat counts append `AND f.is_safe = ?` (0 or 1 depending on active corridor) — ensuring NSFW counts do not leak into the UI when Safe Mode is active.
- **SQL queries MUST use indexed columns** — verified via `EXPLAIN QUERY PLAN` before release; no full table scans.

---

## 4. Dependencies

- **Blocked by**: Epic 02 (Game Management — `game_id` list, `mods_path`, `install_path`), Epic 09 (Object Schema — `object_type` for category distribution), Epic 18 (INI Parser — keybinding extraction), Epic 32 (Dedup Scanner — `duplicate_waste_bytes`).
- **Blocks**: Nothing — read-only analytics feature.
