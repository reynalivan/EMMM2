# Epic 13: Global Dashboard & Executive Insights

**Focus:** Providing a command center (Home) that summarizes data from all managed games, providing usage insights, and quick access to key features using efficient SQL aggregation queries.

## Dependencies

| Direction  | Epic   | Relationship                                         |
| ---------- | ------ | ---------------------------------------------------- |
| ⬆ Upstream | Epic 7 | Dashboard queries filter by Safe Mode state          |
| ⬆ Upstream | Epic 1 | Reads game list for cross-game aggregation           |
| ⬆ Upstream | All    | Aggregates data from mods, collections, games tables |

## Cross-Cutting Requirements

- **Charts:** Use `Recharts` library (per TRD) for pie/bar/line charts.
- **Safe Mode Filter:** All dashboard queries append `AND is_safe = 1` when `appStore.safeMode = true` (dual guard, E7 owns logic).
- **SQL Indexing:** All aggregation queries MUST use indexed columns. No full table scans.
- **TanStack Query:** Dashboard data cached with `staleTime: 30_000ms` (30s). Manual refresh button for instant invalidation.
- **Loading State:** Show skeleton shimmer while data loads (< 200ms target).

---

## 1. User Stories & Acceptance Criteria

### US-13.1: Global Overview Tiles

**As a** user, **I want to** see a summary of statistics from across my entire mod ecosystem, **So that** I know how much content I have without having to check each game individually.

- **Acceptance Criteria:**
  - **Global Counter:** Displays the total mods (Enabled vs Disabled) across the entire database.
  - **Storage Impact:** Displays the total disk usage by the `/Mods` folders.
  - **Duplicate Warning:** If duplicates are detected (Epic 9), display an interactive banner: _"150MB Wasted on Duplicates"_.

### US-13.2: Analytics & Charts

**As a** mod collector, **I want to** view visualizations of my mod distribution (e.g., Characters vs Weapons), **So that** I know which categories are the most dominant.

- **Acceptance Criteria:**
  - **Category Chart:** Interactive Pie Chart that divides mods based on `object_type` (Character/Weapon/UI).
  - **Game Distribution:** Bar Chart comparing the number of mods across games (Genshin vs Star Rail vs ZZZ).

### US-13.3: Quick Activity Hub

**As a** busy user, **I want** quick access to mods I recently added or the last game I played, **So that** I can jump right into action.

- **Acceptance Criteria:**
  - **Recently Added:** List of the 5 latest mods (sorted by `date_added` DESC).
  - **Last Played:** A "Quick Play" button for the last game run.
  - **Active Key Mapping:** Dynamic widget displaying shortcuts from currently active mods (e.g., "Raiden: F6").

---

## 2. Technical Specifications (Rust/SQLx Implementation)

### A. Dashboard Data Aggregation

Uses SQL queries optimized for _sub-millisecond_ dashboard performance.

```rust
use sqlx::{SqlitePool, FromRow};
use serde::Serialize;

#[derive(Serialize, FromRow)]
struct DashboardStats {
    total_mods: i64,
    enabled_mods: i64,
    total_size_bytes: i64,
    duplicate_waste_bytes: Option<i64>,
}

#[derive(Serialize, FromRow)]
struct CategoryDistribution {
    category: String, // 'Character', 'Weapon'
    count: i64,
}

async fn fetch_dashboard_stats(db: &SqlitePool) -> Result<DashboardStats, AppError> {
    // 1. Basic Counts
    let stats = sqlx::query_as!(
        DashboardStats,
        r#"
        SELECT
            COUNT(*) as total_mods,
            SUM(CASE WHEN is_enabled = 1 THEN 1 ELSE 0 END) as enabled_mods,
            SUM(size_bytes) as total_size_bytes,
            (SELECT SUM(wasted_size) FROM duplicates) as duplicate_waste_bytes
        FROM mods
        "#
    ).fetch_one(db).await?;

    Ok(stats)
}

async fn fetch_distribution(db: &SqlitePool) -> Result<Vec<CategoryDistribution>, AppError> {
    // 2. Group By Object Type
    let dist = sqlx::query_as!(
        CategoryDistribution,
        r#"
        SELECT object_type as category, COUNT(*) as count
        FROM mods
        GROUP BY object_type
        "#
    ).fetch_all(db).await?;

    Ok(dist)
}
```

### B. Recent Activity Widget

Retrieves the latest mod data with limitations.

```rust
#[derive(Serialize, FromRow)]
struct RecentMod {
    id: String,
    name: String,
    thumbnail_path: Option<String>,
    date_added: String,
}

async fn fetch_recents(db: &SqlitePool) -> Result<Vec<RecentMod>, AppError> {
    let recents = sqlx::query_as!(
        RecentMod,
        "SELECT id, name, thumbnail_path, date_added FROM mods ORDER BY date_added DESC LIMIT 5"
    ).fetch_all(db).await?;

    Ok(recents)
}
```

---

## 3. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Stats Loading**: Dashboard opens → Numbers appear instantly matching physical folder count.
- [ ] **Chart Rendering**: Distribution Pie Chart renders correctly via `Recharts` with > 0 segments.
- [ ] **Quick Action**: Click "Last Played: Genshin" → Launcher executes immediately.

### 2. Negative Cases (Error Handling)

- [ ] **Zero Data**: Fresh Install → Dashboard shows "Welcome! Add your first Game" empty state (No NaN).
- [ ] **DB Lock**: Aggregation query fails → UI shows skeleton loader → Retry button appears.
- [ ] **Loading State**: While queries run, shimmer skeletons shown (< 200ms target).

### 3. Edge Cases (Stability)

- [ ] **Privacy Sync**: Switch to SFW Mode → "Recently Added" filters out NSFW mods instantly.
- [ ] **Large Library**: 10k Mods → Dashboard Aggregation query in **< 50ms** (Indexed).
- [ ] **Cross-Game Totals**: With 4 games configured → totals correctly sum across all games.

### 4. Technical Metrics

- [ ] **Query Speed**: All queries use Index Scan, no Full Table Scan. Verified via `EXPLAIN QUERY PLAN`.
- [ ] **Memory**: Dashboard JSON payload < 5KB.
- [ ] **Accessibility**: All charts have ARIA labels. Stats tiles are screen-reader friendly.
