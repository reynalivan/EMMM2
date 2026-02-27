use serde::Serialize;
use sqlx::SqlitePool;

// ── Response Structs ────────────────────────────────────────────────────────

/// Global overview statistics for the dashboard tiles.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DashboardStats {
    pub total_mods: i64,
    pub enabled_mods: i64,
    pub disabled_mods: i64,
    pub total_size_bytes: i64,
    pub total_games: i64,
    pub total_collections: i64,
}

/// A single slice of the category distribution pie chart.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CategorySlice {
    pub category: String,
    pub count: i64,
}

/// A single bar of the game distribution bar chart.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct GameSlice {
    pub game_id: String,
    pub game_name: String,
    pub count: i64,
}

/// A recently indexed mod for the activity widget.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RecentMod {
    pub id: String,
    pub name: String,
    pub game_name: String,
    pub object_name: Option<String>,
    pub indexed_at: Option<String>,
}

// ── Queries ─────────────────────────────────────────────────────────────────

/// Fetch global overview stats (total/enabled/disabled mods, size, games, collections).
/// When `safe_mode` is true, only `is_safe = 1` mods are counted.
pub async fn fetch_global_stats(
    pool: &SqlitePool,
    safe_mode: bool,
) -> Result<DashboardStats, sqlx::Error> {
    let safe_clause = if safe_mode { "AND m.is_safe = 1" } else { "" };

    let query = format!(
        r#"
        SELECT
            COALESCE(COUNT(*), 0)                                          AS total_mods,
            COALESCE(SUM(CASE WHEN m.status = 'ENABLED' THEN 1 ELSE 0 END), 0) AS enabled_mods,
            COALESCE(SUM(CASE WHEN m.status != 'ENABLED' THEN 1 ELSE 0 END), 0) AS disabled_mods,
            COALESCE(SUM(CASE WHEN m.size_bytes > 0 THEN m.size_bytes ELSE 0 END), 0) AS total_size_bytes,
            (SELECT COUNT(*) FROM games)                                   AS total_games,
            (SELECT COUNT(*) FROM collections)                             AS total_collections
        FROM mods m
        WHERE 1=1 {safe_clause}
        "#,
    );

    let row = sqlx::query_as::<_, DashboardStats>(&query)
        .fetch_one(pool)
        .await?;

    Ok(row)
}

/// Fetch duplicate waste bytes from dedup scanner results.
/// Returns the sum of `size_bytes` for non-primary members in pending groups.
pub async fn fetch_duplicate_waste(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(m.size_bytes), 0)
        FROM dedup_group_members dgm
        JOIN dedup_groups dg ON dg.id = dgm.group_id
        JOIN mods m ON m.id = dgm.folder_id
        WHERE dgm.is_primary = 0
          AND dg.resolution_status = 'pending'
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Fetch mod counts grouped by `object_type` for the category distribution chart.
pub async fn fetch_category_distribution(
    pool: &SqlitePool,
    safe_mode: bool,
) -> Result<Vec<CategorySlice>, sqlx::Error> {
    let safe_clause = if safe_mode { "AND m.is_safe = 1" } else { "" };

    let query = format!(
        r#"
        SELECT
            COALESCE(m.object_type, 'Uncategorized') AS category,
            COUNT(*) AS count
        FROM mods m
        WHERE 1=1 {safe_clause}
        GROUP BY COALESCE(m.object_type, 'Uncategorized')
        ORDER BY count DESC
        "#,
    );

    sqlx::query_as::<_, CategorySlice>(&query)
        .fetch_all(pool)
        .await
}

/// Fetch mod counts grouped by game for the game distribution bar chart.
pub async fn fetch_game_distribution(
    pool: &SqlitePool,
    safe_mode: bool,
) -> Result<Vec<GameSlice>, sqlx::Error> {
    let safe_clause = if safe_mode { "AND m.is_safe = 1" } else { "" };

    let query = format!(
        r#"
        SELECT
            g.id   AS game_id,
            g.name AS game_name,
            COUNT(m.id) AS count
        FROM games g
        LEFT JOIN mods m ON m.game_id = g.id {safe_clause}
        GROUP BY g.id, g.name
        ORDER BY count DESC
        "#,
    );

    sqlx::query_as::<_, GameSlice>(&query).fetch_all(pool).await
}

/// Fetch the N most recently indexed mods for the activity widget.
pub async fn fetch_recent_mods(
    pool: &SqlitePool,
    safe_mode: bool,
    limit: i64,
) -> Result<Vec<RecentMod>, sqlx::Error> {
    let safe_clause = if safe_mode { "AND m.is_safe = 1" } else { "" };

    let query = format!(
        r#"
        SELECT
            m.id,
            m.actual_name AS name,
            g.name        AS game_name,
            o.name        AS object_name,
            m.indexed_at
        FROM mods m
        JOIN games g ON g.id = m.game_id
        LEFT JOIN objects o ON o.id = m.object_id
        WHERE 1=1 {safe_clause}
        ORDER BY m.indexed_at DESC
        LIMIT ?
        "#,
    );

    sqlx::query_as::<_, RecentMod>(&query)
        .bind(limit)
        .fetch_all(pool)
        .await
}

#[cfg(test)]
#[path = "tests/dashboard_repo_tests.rs"]
mod tests;
