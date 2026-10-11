use crate::modules::dashboard::domain::dashboard::{
    CategorySlice, DashboardStats, GameSlice, RecentMod,
};
use sqlx::{Row, SqlitePool};
use std::path::{Path, MAIN_SEPARATOR_STR};

// ── Response Structs ────────────────────────────────────────────────────────

// ── Queries ─────────────────────────────────────────────────────────────────

/// Fetch global overview stats (total/enabled/disabled mods, games, collections).
pub async fn fetch_global_stats(pool: &SqlitePool) -> Result<DashboardStats, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            COALESCE(COUNT(*), 0)                                          AS total_mods,
            COALESCE(SUM(CASE WHEN m.status = 1 THEN 1 ELSE 0 END), 0) AS enabled_mods,
            COALESCE(SUM(CASE WHEN m.status = 0 THEN 1 ELSE 0 END), 0) AS disabled_mods,
            (SELECT COUNT(*) FROM games)                                   AS total_games,
            (SELECT COUNT(*) FROM collections)                             AS total_collections
        FROM mods m
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(DashboardStats {
        total_mods: row.try_get("total_mods")?,
        enabled_mods: row.try_get("enabled_mods")?,
        disabled_mods: row.try_get("disabled_mods")?,
        total_games: row.try_get("total_games")?,
        total_collections: row.try_get("total_collections")?,
    })
}

/// Fetch duplicate waste bytes from dedup scanner results.
/// Uses the optimizer's measured report; routine indexing does not size assets.
pub async fn fetch_duplicate_waste(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    #[derive(serde::Deserialize)]
    struct MeasuredGroup {
        members: Vec<MeasuredMember>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MeasuredMember {
        mod_id: Option<String>,
        total_size_bytes: u64,
    }

    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT dg.reasons_json, jobs.root_path, games.mods_path,
            (SELECT json_group_array(members.folder_id)
             FROM dedup_group_members members
             JOIN mods ON mods.id = members.folder_id
             WHERE members.group_id = dg.id AND mods.game_id = jobs.game_id)
        FROM dedup_groups dg
        JOIN dedup_jobs jobs ON jobs.id = dg.job_id
        JOIN games ON games.id = jobs.game_id
        WHERE dg.resolution_status = 'pending'
          AND jobs.status = 'completed'
          AND jobs.root_path IS NOT NULL AND jobs.root_path != ''
          AND games.mods_path != ''
          AND jobs.id = (
              SELECT latest.id FROM dedup_jobs latest
              WHERE latest.game_id = jobs.game_id AND latest.status = 'completed'
              ORDER BY latest.completed_at DESC, latest.started_at DESC LIMIT 1
          )
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut waste = 0_i64;
    for (json, stored_root, configured_root, live_ids_json) in rows {
        if crate::shared::path_key::exact_location_key_for_path(Path::new(&stored_root))
            != crate::shared::path_key::exact_location_key_for_path(Path::new(&configured_root))
        {
            continue;
        }
        let group: MeasuredGroup =
            serde_json::from_str(&json).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        let live_ids: std::collections::HashSet<String> = serde_json::from_str(&live_ids_json)
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        let surviving_members = group.members.iter().filter(|member| {
            member
                .mod_id
                .as_ref()
                .is_none_or(|id| live_ids.contains(id))
        });
        for member in surviving_members.skip(1) {
            let measured = i64::try_from(member.total_size_bytes)
                .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
            waste = waste.checked_add(measured).ok_or_else(|| {
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Duplicate waste exceeds the supported byte range",
                )))
            })?;
        }
    }
    Ok(waste)
}

/// Fetch enabled mods together with the catalog metadata used by the
/// Active Key Mapping widget. The mod classification is a denormalized
/// projection, so fall back to the owning object for older rows.
pub async fn fetch_enabled_mods_for_keybindings(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String, Option<String>, Option<String>)>, sqlx::Error> {
    let rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT
            m.actual_name,
            m.folder_path,
            COALESCE(m.object_type, o.object_type) AS object_type,
            o.matched_alias_name
        FROM mods m
        LEFT JOIN objects o ON o.id = m.object_id
        WHERE m.game_id = ?
          AND m.status = 1
        ORDER BY m.folder_path_key
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|(_, folder_path, _, _)| {
            crate::modules::workspace::api::is_effectively_enabled_path(folder_path)
        })
        .collect())
}

/// Fetch mod counts grouped by `object_type` for the category distribution chart.
pub async fn fetch_category_distribution(
    pool: &SqlitePool,
) -> Result<Vec<CategorySlice>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            COALESCE(m.object_type, 'Uncategorized') AS category,
            COUNT(*) AS count
        FROM mods m
        GROUP BY COALESCE(m.object_type, 'Uncategorized')
        ORDER BY count DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(CategorySlice {
                category: row.try_get("category")?,
                count: row.try_get("count")?,
            })
        })
        .collect()
}

/// Fetch mod counts grouped by game for the game distribution bar chart.
pub async fn fetch_game_distribution(pool: &SqlitePool) -> Result<Vec<GameSlice>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            g.id   AS game_id,
            g.name AS game_name,
            COUNT(m.id) AS count
        FROM games g
        LEFT JOIN mods m ON m.game_id = g.id
        GROUP BY g.id, g.name
        ORDER BY count DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(GameSlice {
                game_id: row.try_get("game_id")?,
                game_name: row.try_get("game_name")?,
                count: row.try_get("count")?,
            })
        })
        .collect()
}

/// Fetch the N most recently indexed mods for the activity widget.
pub async fn fetch_recent_mods(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<RecentMod>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            m.id,
            m.game_id,
            m.actual_name AS name,
            g.name        AS game_name,
            o.name AS object_name,
            m.folder_path,
            COALESCE(NULLIF(g.mods_path, ''), g.path) AS mods_path,
            m.indexed_at
        FROM mods m
        JOIN games g ON g.id = m.game_id
        LEFT JOIN objects o ON o.id = m.object_id
        ORDER BY m.indexed_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            let folder_path: String = row.try_get("folder_path")?;
            let mods_path: String = row.try_get("mods_path")?;
            let stored_folder_path = Path::new(&folder_path);
            let resolved_folder_path = if stored_folder_path.is_absolute()
                || stored_folder_path.has_root()
            {
                folder_path
            } else {
                let normalized_relative_path = folder_path.replace(['/', '\\'], MAIN_SEPARATOR_STR);
                Path::new(&mods_path)
                    .join(normalized_relative_path)
                    .to_string_lossy()
                    .to_string()
            };
            Ok(RecentMod {
                id: row.try_get("id")?,
                game_id: row.try_get("game_id")?,
                name: row.try_get("name")?,
                game_name: row.try_get("game_name")?,
                object_name: row.try_get("object_name")?,
                folder_path: resolved_folder_path,
                indexed_at: row.try_get("indexed_at")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
