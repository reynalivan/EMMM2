//! Multi-row reads: mod sets scoped by game or object.

use std::collections::{HashMap, HashSet};

use super::types::{ModSubtreeEntry, ReconcileModRow};
use crate::modules::system::domain::mod_path::ModFolderPath;
use crate::shared::path_key::folder_path_key;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::path::Path;

fn is_effectively_enabled_path(folder_path: &str) -> bool {
    !folder_path
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .any(crate::modules::workspace::domain::normalizer::is_disabled_folder)
}

pub async fn get_rows_for_reconcile(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<ReconcileModRow>, sqlx::Error> {
    sqlx::query_as::<_, ReconcileModRow>(
        "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, safety_source, object_type, filesystem_identity, size_bytes FROM mods WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
}

fn push_root_key_predicate<'a>(
    query: &mut QueryBuilder<'a, Sqlite>,
    root_keys: &'a [String],
    column: &str,
) {
    query.push(" AND (");
    for (position, root_key) in root_keys.iter().enumerate() {
        if position > 0 {
            query.push(" OR ");
        }
        let descendant_start = format!("{root_key}/");
        let descendant_end = format!("{root_key}0");
        query
            .push("(")
            .push(column)
            .push(" = ")
            .push_bind(root_key)
            .push(" OR (")
            .push(column)
            .push(" >= ")
            .push_bind(descendant_start)
            .push(" AND ")
            .push(column)
            .push(" < ")
            .push_bind(descendant_end)
            .push("))");
    }
    query.push(")");
}

/// Indexed rows for only the roots and physical identities participating in a
/// scoped disk projection. The range predicate keeps descendant lookup on the
/// `(game_id, folder_path_key)` index without wildcard scans.
pub async fn get_rows_for_reconcile_scope(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    root_keys: &[String],
    filesystem_identities: &[String],
) -> Result<Vec<ReconcileModRow>, sqlx::Error> {
    // Three binds per root plus game_id remain below SQLite's common limit.
    const ROOT_CHUNK_SIZE: usize = 250;
    const IDENTITY_CHUNK_SIZE: usize = 900;
    let mut rows = Vec::new();
    let mut seen_ids = HashSet::new();

    for keys in root_keys.chunks(ROOT_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, safety_source, object_type, filesystem_identity, size_bytes FROM mods WHERE game_id = ",
        );
        query.push_bind(game_id);
        push_root_key_predicate(&mut query, keys, "folder_path_key");
        for row in query
            .build_query_as::<ReconcileModRow>()
            .fetch_all(&mut *conn)
            .await?
        {
            if seen_ids.insert(row.id.clone()) {
                rows.push(row);
            }
        }
    }

    for identities in filesystem_identities.chunks(IDENTITY_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, safety_source, object_type, filesystem_identity, size_bytes FROM mods WHERE game_id = ",
        );
        query.push_bind(game_id);
        query.push(" AND filesystem_identity IN (");
        {
            let mut separated = query.separated(", ");
            for identity in identities {
                separated.push_bind(identity);
            }
        }
        query.push(")");
        for row in query
            .build_query_as::<ReconcileModRow>()
            .fetch_all(&mut *conn)
            .await?
        {
            if seen_ids.insert(row.id.clone()) {
                rows.push(row);
            }
        }
    }

    Ok(rows)
}

pub async fn get_folder_path_keys_for_roots(
    pool: &SqlitePool,
    game_id: &str,
    root_keys: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    const ROOT_CHUNK_SIZE: usize = 250;
    let mut keys = Vec::new();
    for roots in root_keys.chunks(ROOT_CHUNK_SIZE) {
        let mut query =
            QueryBuilder::<Sqlite>::new("SELECT folder_path_key FROM mods WHERE game_id = ");
        query.push_bind(game_id);
        push_root_key_predicate(&mut query, roots, "folder_path_key");
        keys.extend(query.build_query_scalar::<String>().fetch_all(pool).await?);
    }
    Ok(keys)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExactRuntimeMod {
    pub id: String,
    pub folder_path: String,
    pub status: i64,
    pub has_descendants: i64,
}

/// Resolve exact terminal mod rows after a committed rename. The correlated
/// descendant probe uses the `(game_id, folder_path_key)` index and lets the
/// runtime queue distinguish a leaf toggle from a parent toggle without
/// materializing the parent's complete subtree.
pub async fn get_exact_runtime_mods_for_paths(
    pool: &SqlitePool,
    game_id: &str,
    mods_root: &Path,
    folder_paths: &[String],
) -> Result<Vec<ExactRuntimeMod>, sqlx::Error> {
    const PATH_CHUNK_SIZE: usize = 900;
    let mods_root = mods_root.to_string_lossy();
    let keys = folder_paths
        .iter()
        .map(|path| folder_path_key(path, Some(&mods_root)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut rows = Vec::with_capacity(keys.len());
    for chunk in keys.chunks(PATH_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT candidate.id, candidate.folder_path, candidate.status, \
             EXISTS(SELECT 1 FROM mods descendant \
                    WHERE descendant.game_id = candidate.game_id \
                      AND descendant.folder_path_key >= candidate.folder_path_key || '/' \
                      AND descendant.folder_path_key < candidate.folder_path_key || '0' \
                    LIMIT 1) AS has_descendants \
             FROM mods candidate WHERE candidate.game_id = ",
        );
        query
            .push_bind(game_id)
            .push(" AND candidate.folder_path_key IN (");
        {
            let mut separated = query.separated(", ");
            for key in chunk {
                separated.push_bind(key);
            }
        }
        query.push(")");
        rows.extend(
            query
                .build_query_as::<ExactRuntimeMod>()
                .fetch_all(pool)
                .await?,
        );
    }
    Ok(rows)
}

#[derive(Clone, Copy)]
pub struct SafetyClassification {
    pub is_safe: bool,
    pub is_classified: bool,
}

/// Last-known safety classification keyed by the canonical relative mod path.
pub async fn get_safety_by_folder_path_key(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<HashMap<String, SafetyClassification>, sqlx::Error> {
    let rows: Vec<(String, bool, String)> = sqlx::query_as(
        "SELECT folder_path_key, COALESCE(is_safe, 1), COALESCE(safety_source, 'unknown') FROM mods WHERE game_id = ?",
    )
            .bind(game_id)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(key, is_safe, source)| {
            (
                key,
                SafetyClassification {
                    is_safe,
                    is_classified: source != crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN,
                },
            )
        })
        .collect())
}

/// Returns classifications for one folder and its descendants. The LIKE value
/// escapes user-controlled path characters so `%` and `_` remain literal path
/// bytes rather than widening the requested subtree.
pub async fn get_safety_for_folder_subtree(
    pool: &SqlitePool,
    game_id: &str,
    folder_path_key: &str,
) -> Result<HashMap<String, SafetyClassification>, sqlx::Error> {
    let escaped_key = folder_path_key
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let descendants = format!("{escaped_key}/%");
    let rows: Vec<(String, bool, String)> = sqlx::query_as(
        "SELECT folder_path_key, COALESCE(is_safe, 1), COALESCE(safety_source, 'unknown') FROM mods WHERE game_id = ? AND (folder_path_key = ? OR folder_path_key LIKE ? ESCAPE '\\')",
    )
    .bind(game_id)
    .bind(folder_path_key)
    .bind(descendants)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(key, is_safe, source)| {
            (
                key,
                SafetyClassification {
                    is_safe,
                    is_classified: source != crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN,
                },
            )
        })
        .collect())
}

/// Indexed terminal mods beneath one folder. Disk reconcile runs before a
/// workspace switch, so these rows are a fresh projection of the filesystem
/// used only to describe a pending parent-enable operation.
pub async fn get_mods_for_folder_subtree(
    pool: &SqlitePool,
    game_id: &str,
    folder_path_key: &str,
) -> Result<Vec<ModSubtreeEntry>, sqlx::Error> {
    let escaped_key = folder_path_key
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let descendants = format!("{escaped_key}/%");
    sqlx::query_as::<_, ModSubtreeEntry>(
        "SELECT actual_name, folder_path FROM mods
         WHERE game_id = ? AND (folder_path_key = ? OR folder_path_key LIKE ? ESCAPE '\\')
         ORDER BY folder_path_key",
    )
    .bind(game_id)
    .bind(folder_path_key)
    .bind(descendants)
    .fetch_all(pool)
    .await
}

fn push_randomizer_scope(
    query: &mut QueryBuilder<'_, Sqlite>,
    category_names: &[&'static str],
    include_unclassified: bool,
) {
    query.push(" AND (");
    if !category_names.is_empty() {
        query.push("o.object_type IN (");
        {
            let mut separated = query.separated(", ");
            for category in category_names {
                separated.push_bind(*category);
            }
        }
        query.push(")");
    }
    if include_unclassified {
        if !category_names.is_empty() {
            query.push(" OR ");
        }
        query.push("COALESCE(o.object_type, '') NOT IN ('Character', 'Weapon', 'UI', 'Other')");
    }
    query.push(")");
}

/// All scoped Object mods for one game, in one query. Randomizer eligibility
/// that depends on path semantics is applied by the application layer.
pub async fn get_randomizer_candidates(
    pool: &SqlitePool,
    game_id: &str,
    category_names: &[&'static str],
    include_unclassified: bool,
) -> Result<Vec<super::types::RandomizerModCandidate>, sqlx::Error> {
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT m.id, m.object_id, o.name AS object_name, m.actual_name, m.folder_path, \
                NULLIF(o.object_type, '') AS object_type, o.randomizer_mode, m.status, COALESCE(m.is_safe, 1) AS is_safe \
         FROM mods m \
         INNER JOIN objects o ON o.id = m.object_id AND o.game_id = m.game_id \
         WHERE m.game_id = ",
    );
    query.push_bind(game_id);
    push_randomizer_scope(&mut query, category_names, include_unclassified);
    query.push(" ORDER BY o.name, m.folder_path_key");
    query
        .build_query_as::<super::types::RandomizerModCandidate>()
        .fetch_all(pool)
        .await
}

/// Randomizer candidates selected by id, constrained to the active game and
/// scope. Dynamic placeholders are bound, never interpolated.
pub async fn get_randomizer_candidates_by_ids(
    pool: &SqlitePool,
    game_id: &str,
    mod_ids: &[String],
    category_names: &[&'static str],
    include_unclassified: bool,
) -> Result<Vec<super::types::RandomizerModCandidate>, sqlx::Error> {
    if mod_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT m.id, m.object_id, o.name AS object_name, m.actual_name, m.folder_path, \
                NULLIF(o.object_type, '') AS object_type, o.randomizer_mode, m.status, COALESCE(m.is_safe, 1) AS is_safe \
         FROM mods m \
         INNER JOIN objects o ON o.id = m.object_id AND o.game_id = m.game_id \
         WHERE m.game_id = ",
    );
    query.push_bind(game_id);
    push_randomizer_scope(&mut query, category_names, include_unclassified);
    query.push(" AND m.id IN (");
    {
        let mut separated = query.separated(", ");
        for mod_id in mod_ids {
            separated.push_bind(mod_id);
        }
    }
    query.push(") ORDER BY o.name, m.folder_path_key");

    query
        .build_query_as::<super::types::RandomizerModCandidate>()
        .fetch_all(pool)
        .await
}

pub async fn get_enabled_mods_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ModFolderPath>, sqlx::Error> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ? AND status = 1")
            .bind(game_id)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .filter(|path| is_effectively_enabled_path(path))
        .map(ModFolderPath::from_stored)
        .collect())
}

/// Canonical stored paths for every indexed terminal mod in one game.
pub async fn get_folder_paths_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ? ORDER BY folder_path_key")
        .bind(game_id)
        .fetch_all(pool)
        .await
}

/// Canonical keys for the currently indexed terminal mods in one game.
/// Used to limit storage metadata walks to newly discovered folders.
pub async fn get_folder_path_keys_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT folder_path_key FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(pool)
        .await
}

pub async fn get_enabled_siblings_paths(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_mod_id: Option<&str>,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT folder_path FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 1
         AND (? IS NULL OR id != ?)",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_mod_id)
    .bind(exclude_mod_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|path| is_effectively_enabled_path(path))
        .collect())
}

/// All terminal mod paths owned by an Object, including rows currently hidden
/// below a disabled ancestor. Randomizer exclusive activation needs this to
/// prevent an ancestor rename from exposing sibling mods at the same time.
pub async fn get_object_mod_paths(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
    exclude_mod_id: Option<&str>,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT folder_path FROM mods
         WHERE object_id = ? AND game_id = ? AND (? IS NULL OR id != ?)
         ORDER BY folder_path_key",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_mod_id)
    .bind(exclude_mod_id)
    .fetch_all(pool)
    .await
}

/// Effective active mod names for one Object. A terminal row under a disabled
/// ancestor is not runtime-active even if its stale projection says enabled.
pub async fn get_effectively_enabled_mod_names(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT actual_name, folder_path FROM mods WHERE game_id = ? AND object_id = ? AND status = 1 ORDER BY folder_path_key",
    )
    .bind(game_id)
    .bind(object_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, path)| is_effectively_enabled_path(path))
        .map(|(name, _)| name)
        .collect())
}

pub async fn get_recent_randomizer_history(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<HashMap<String, Vec<String>>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT object_id, mod_id FROM (
             SELECT object_id, mod_id,
                    ROW_NUMBER() OVER (PARTITION BY object_id ORDER BY applied_at DESC, rowid DESC) AS position
             FROM randomizer_applied_history WHERE game_id = ?
         ) WHERE position <= 3 ORDER BY object_id, position",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    let mut history = HashMap::<String, Vec<String>>::new();
    for (object_id, mod_id) in rows {
        history.entry(object_id).or_default().push(mod_id);
    }
    Ok(history)
}

pub async fn record_randomizer_history(
    pool: &SqlitePool,
    game_id: &str,
    selections: &[(String, String)],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for (object_id, mod_id) in selections {
        sqlx::query(
            "DELETE FROM randomizer_applied_history
             WHERE game_id = ? AND object_id = ? AND mod_id = ?",
        )
        .bind(game_id)
        .bind(object_id)
        .bind(mod_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO randomizer_applied_history (game_id, object_id, mod_id, applied_at)
             VALUES (?, ?, ?, strftime('%Y-%m-%d %H:%M:%f', 'now'))",
        )
        .bind(game_id)
        .bind(object_id)
        .bind(mod_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM randomizer_applied_history
             WHERE game_id = ? AND object_id = ? AND mod_id NOT IN (
                 SELECT mod_id FROM randomizer_applied_history
                 WHERE game_id = ? AND object_id = ?
                 ORDER BY applied_at DESC, rowid DESC LIMIT 3
             )",
        )
        .bind(game_id)
        .bind(object_id)
        .bind(game_id)
        .bind(object_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn get_enabled_duplicates(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_mod_id: Option<&str>,
) -> Result<Vec<(String, ModFolderPath, String)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, folder_path, actual_name FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 1
         AND (? IS NULL OR id != ?)",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_mod_id)
    .bind(exclude_mod_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, path, _)| is_effectively_enabled_path(path))
        .map(|(id, path, name)| (id, ModFolderPath::from_stored(path), name))
        .collect())
}

pub async fn get_enabled_mods_names_and_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, ModFolderPath)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT actual_name, folder_path FROM mods WHERE game_id = ? AND status = 1",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, path)| is_effectively_enabled_path(path))
        .map(|(name, path)| (name, ModFolderPath::from_stored(path)))
        .collect())
}

#[cfg(test)]
#[path = "tests/listing_tests.rs"]
mod tests;

pub async fn get_all_mods_id_and_paths_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<(String, ModFolderPath, bool)>, sqlx::Error> {
    let rows: Vec<(String, String, bool)> =
        sqlx::query_as("SELECT id, folder_path, COALESCE(is_safe, 1) FROM mods WHERE game_id = ?")
            .bind(game_id)
            .fetch_all(conn)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, path, is_safe)| (id, ModFolderPath::from_stored(path), is_safe))
        .collect())
}
