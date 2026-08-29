use crate::domain::conflicts::WhitelistEntry;
use crate::domain::errors::ScannerError;
use crate::types::dup_scan::{DupScanGroup, DupScanReport};
use sqlx::SqlitePool;

pub async fn persist_completed_report(
    pool: &SqlitePool,
    report: &DupScanReport,
) -> Result<(), ScannerError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM dedup_jobs WHERE game_id = ?")
        .bind(&report.game_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "INSERT INTO dedup_jobs (id, game_id, status, completed_at) \
         VALUES (?, ?, 'completed', CURRENT_TIMESTAMP)",
    )
    .bind(&report.scan_id)
    .bind(&report.game_id)
    .execute(&mut *transaction)
    .await?;

    for group in &report.groups {
        let group_json = serde_json::to_string(group)?;
        sqlx::query(
            "INSERT INTO dedup_groups (id, job_id, reasons_json, resolution_status) \
             VALUES (?, ?, ?, 'pending')",
        )
        .bind(&group.group_id)
        .bind(&report.scan_id)
        .bind(group_json)
        .execute(&mut *transaction)
        .await?;

        for (index, member) in group.members.iter().enumerate() {
            let Some(mod_id) = member.mod_id.as_deref() else {
                continue;
            };
            sqlx::query(
                "INSERT INTO dedup_group_members \
                 (id, group_id, folder_id, signals_json, is_primary) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&group.group_id)
            .bind(mod_id)
            .bind(serde_json::to_string(member)?)
            .bind(index == 0)
            .execute(&mut *transaction)
            .await?;
        }
    }

    transaction.commit().await?;
    Ok(())
}

pub async fn load_latest_completed_report(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<DupScanReport>, ScannerError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT jobs.id, games.mods_path \
         FROM dedup_jobs jobs \
         JOIN games ON games.id = jobs.game_id \
         WHERE jobs.game_id = ? AND jobs.status = 'completed' \
         ORDER BY jobs.completed_at DESC, jobs.started_at DESC LIMIT 1",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await?;
    let Some((scan_id, root_path)) = row else {
        return Ok(None);
    };

    let group_rows: Vec<String> = sqlx::query_scalar(
        "SELECT reasons_json FROM dedup_groups \
         WHERE job_id = ? AND resolution_status = 'pending' ORDER BY created_at, id",
    )
    .bind(&scan_id)
    .fetch_all(pool)
    .await?;
    let groups: Vec<DupScanGroup> = group_rows
        .into_iter()
        .map(|json| serde_json::from_str(&json))
        .collect::<Result<_, _>>()?;
    let total_members = groups.iter().map(|group| group.members.len()).sum();

    Ok(Some(DupScanReport {
        scan_id,
        game_id: game_id.to_string(),
        root_path,
        total_groups: groups.len(),
        total_members,
        groups,
    }))
}

pub async fn load_pending_group(
    pool: &SqlitePool,
    game_id: &str,
    group_id: &str,
) -> Result<Option<DupScanGroup>, ScannerError> {
    let group_json: Option<String> = sqlx::query_scalar(
        "SELECT groups.reasons_json \
         FROM dedup_groups groups \
         JOIN dedup_jobs jobs ON jobs.id = groups.job_id \
         WHERE groups.id = ? AND jobs.game_id = ? \
           AND jobs.status = 'completed' AND groups.resolution_status = 'pending'",
    )
    .bind(group_id)
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    group_json
        .map(|json| serde_json::from_str(&json).map_err(ScannerError::from))
        .transpose()
}

pub async fn get_duplicate_whitelist_pairs(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT folder_a_id, folder_b_id FROM duplicate_whitelist WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(pool)
        .await
}

pub async fn get_whitelist_detailed(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<WhitelistEntry>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT 
            w.id,
            w.folder_a_id,
            w.folder_b_id,
            m1.actual_name as folder_a_name,
            m2.actual_name as folder_b_name,
            w.reason,
            w.ignored_at
        FROM duplicate_whitelist w
        JOIN mods m1 ON w.folder_a_id = m1.id
        JOIN mods m2 ON w.folder_b_id = m2.id
        WHERE w.game_id = ?
        ORDER BY w.ignored_at DESC
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_whitelist_entry(pool: &SqlitePool, entry_id: &str) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM duplicate_whitelist WHERE id = ?")
        .bind(entry_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn insert_whitelist_pair(
    pool: &SqlitePool,
    game_id: &str,
    canonical_a: &str,
    canonical_b: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO duplicate_whitelist (id, game_id, folder_a_id, folder_b_id, reason)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(game_id)
    .bind(canonical_a)
    .bind(canonical_b)
    .bind("Manual duplicate ignore")
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_group_status(
    pool: &SqlitePool,
    group_id: &str,
    status: &str,
    set_resolved_at: bool,
) -> Result<u64, sqlx::Error> {
    let query = if set_resolved_at {
        "UPDATE dedup_groups SET resolution_status = ?, resolved_at = CURRENT_TIMESTAMP WHERE id = ?"
    } else {
        "UPDATE dedup_groups SET resolution_status = ? WHERE id = ?"
    };

    let result = sqlx::query(query)
        .bind(status)
        .bind(group_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
#[path = "tests/dedup_repo_test.rs"]
mod tests;
