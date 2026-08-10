//! The parsed MasterDB, cached per game type, with the user's own aliases
//! folded into the bundled entries.

use crate::domain::errors::ScannerError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::services::scanner::deep_matcher;

/// Parsed MasterDB per game type, so a 5 MB JSON is read and parsed once.
#[derive(Default)]
pub struct MasterDbCache(tokio::sync::RwLock<HashMap<String, Arc<deep_matcher::MasterDb>>>);

impl MasterDbCache {
    /// Drop every parsed database so the next scan picks up edited user aliases.
    ///
    /// Clears all game types rather than one: the caller that edits an object
    /// knows its game id, not its game type, and re-parsing costs one JSON read
    /// on the next scan.
    pub async fn invalidate(app: &tauri::AppHandle) {
        use tauri::Manager;
        app.state::<MasterDbCache>().0.write().await.clear();
    }
}

/// The parsed MasterDB for a game type, loading it on first use.
///
/// Returns `None` when the game has no bundled database. This used to be a
/// `db_json: String` parameter: the frontend fetched the whole database, held
/// it, and posted it back on every scan command, which then re-parsed it. The
/// backend has the file — there was never a reason for it to cross IPC.
pub async fn get_cached(
    app: &tauri::AppHandle,
    game_type: i32,
) -> Result<Option<Arc<deep_matcher::MasterDb>>, ScannerError> {
    use tauri::Manager;

    let canonical = crate::services::game::schema_loader::normalize_game_type(game_type);
    let cache = app.state::<MasterDbCache>();

    if let Some(hit) = cache.0.read().await.get(&canonical).cloned() {
        return Ok(Some(hit));
    }

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| ScannerError::Io(format!("failed to resolve resource dir: {error}")))?;
    let db_path = resource_dir
        .join("databases")
        .join(format!("{canonical}.json"));
    if !db_path.exists() {
        return Ok(None);
    }

    let json = std::fs::read_to_string(&db_path)?;
    let mut db = deep_matcher::MasterDb::from_json(&json)?;
    attach_user_aliases(&mut db, &load_user_aliases(&app.state::<sqlx::SqlitePool>()).await);

    let parsed = Arc::new(db);
    cache.0.write().await.insert(canonical, Arc::clone(&parsed));
    Ok(Some(parsed))
}

/// Aliases the user typed on their own objects, grouped by matched entry key.
///
/// The UI has always let users add aliases to an object's custom skins, but
/// they were written to the `objects` table and never read back by the matcher,
/// so they changed nothing. This is the read half.
async fn load_user_aliases(pool: &sqlx::SqlitePool) -> HashMap<String, Vec<String>> {
    let blobs = match crate::repo::object_repo::get_user_alias_blobs(pool).await {
        Ok(rows) => rows,
        Err(error) => {
            log::warn!("user aliases unavailable, matching with bundled aliases only: {error}");
            return HashMap::new();
        }
    };

    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for (entry_key, json) in blobs {
        let skins: Vec<deep_matcher::CustomSkin> = match serde_json::from_str(&json) {
            Ok(parsed) => parsed,
            Err(error) => {
                log::warn!("skipping unreadable custom_skins for entry_key={entry_key}: {error}");
                continue;
            }
        };
        let aliases = skins.into_iter().flat_map(|skin| skin.aliases);
        grouped.entry(entry_key).or_default().extend(aliases);
    }
    grouped
}

/// Fold user aliases into their matching entries.
///
/// `keywords` and `indexes` are built from name and tags only, so attaching a
/// skin needs no rebuild — the alias stage reads `custom_skins` at match time.
/// This puts user aliases on exactly the same footing as bundled ones.
fn attach_user_aliases(
    db: &mut deep_matcher::MasterDb,
    by_entry_key: &HashMap<String, Vec<String>>,
) {
    let mut attached = 0usize;
    for entry in &mut db.entries {
        let key = crate::services::scanner::sync::helpers::canonical_entry_key(&entry.name);
        let Some(aliases) = by_entry_key.get(&key) else {
            continue;
        };

        let known: HashSet<String> = entry
            .custom_skins
            .iter()
            .flat_map(|skin| skin.aliases.iter())
            .map(|alias| alias.trim().to_lowercase())
            .collect();

        let mut fresh: Vec<String> = Vec::new();
        for alias in aliases {
            let normalized = alias.trim().to_lowercase();
            if normalized.is_empty() || known.contains(&normalized) || fresh.contains(alias) {
                continue;
            }
            fresh.push(alias.clone());
        }

        if fresh.is_empty() {
            continue;
        }
        attached += fresh.len();
        entry.custom_skins.push(deep_matcher::CustomSkin {
            name: "User".to_string(),
            aliases: fresh,
            thumbnail_skin_path: None,
            rarity: None,
        });
    }

    // Aliases whose entry key matches nothing bundled are dropped: the matcher
    // can only ever return a bundled entry.
    log::debug!(
        "master_db: attached {attached} user alias(es) from {} keyed object(s)",
        by_entry_key.len()
    );
}

#[cfg(test)]
#[path = "../tests/master_db_cache_tests.rs"]
mod master_db_cache_tests;
