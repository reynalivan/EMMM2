//! The parsed MasterDB, cached per game type, with the user's own aliases
//! folded into the bundled entries.

use crate::shared::errors::ScannerError;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::modules::matching::application::deep_matcher;

static ALIAS_SOURCE_REVISION: AtomicU64 = AtomicU64::new(0);

pub(crate) fn committed_object_removal() {
    ALIAS_SOURCE_REVISION.fetch_add(1, Ordering::AcqRel);
}

fn alias_source_revision() -> u64 {
    ALIAS_SOURCE_REVISION.load(Ordering::Acquire)
}

/// Parsed MasterDB per game type, so a 5 MB JSON is read and parsed once.
#[derive(Default)]
pub struct MasterDbCache(tokio::sync::RwLock<CacheState>);

#[derive(Default)]
struct CacheState {
    generation: u64,
    entries: HashMap<String, (u64, Arc<deep_matcher::MasterDb>)>,
}

impl MasterDbCache {
    /// Drop every parsed database so the next scan picks up edited user aliases.
    ///
    /// Clears all game types rather than one: the caller that edits an object
    /// knows its game id, not its game type, and re-parsing costs one JSON read
    /// on the next scan.
    pub async fn invalidate(app: &tauri::AppHandle) {
        use tauri::Manager;
        app.state::<MasterDbCache>().clear().await;
    }

    async fn clear(&self) {
        let mut state = self.0.write().await;
        state.generation = state.generation.wrapping_add(1);
        state.entries.clear();
    }

    async fn get_or_load<F, Fut>(
        &self,
        canonical: &str,
        source_revision: impl Fn() -> u64,
        load: F,
    ) -> Result<Arc<deep_matcher::MasterDb>, ScannerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<deep_matcher::MasterDb, ScannerError>>,
    {
        loop {
            let alias_revision = source_revision();
            let generation = {
                let state = self.0.read().await;
                if source_revision() != alias_revision {
                    continue;
                }
                if let Some((_, hit)) = state
                    .entries
                    .get(canonical)
                    .filter(|(revision, _)| *revision == alias_revision)
                {
                    return Ok(Arc::clone(hit));
                }
                state.generation
            };
            let loaded = load().await;
            let mut state = self.0.write().await;
            // Loading awaits SQLite. Do not publish a read from before an invalidation.
            if state.generation != generation || source_revision() != alias_revision {
                continue;
            }
            let parsed = Arc::new(loaded?);
            let (_, hit) = state
                .entries
                .entry(canonical.to_string())
                .and_modify(|cached| {
                    if cached.0 != alias_revision {
                        *cached = (alias_revision, Arc::clone(&parsed));
                    }
                })
                .or_insert((alias_revision, parsed));
            return Ok(Arc::clone(hit));
        }
    }
}

/// The parsed MasterDB for a game type, loading it on first use.
///
/// When the optional catalog pack is not installed, returns an empty database.
/// Import and classification can then continue with source inspection and
/// manual choices instead of failing before the review screen opens.
pub async fn get_cached(
    app: &tauri::AppHandle,
    game_type: i32,
) -> Result<Arc<deep_matcher::MasterDb>, ScannerError> {
    use tauri::Manager;

    let canonical =
        crate::modules::games::application::game::schema_loader::normalize_game_type(game_type);
    let cache = app.state::<MasterDbCache>();

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        ScannerError::Io(format!("failed to resolve app data directory: {error}"))
    })?;
    cache
        .get_or_load(&canonical, alias_source_revision, || async {
            let mut entries = super::load_master_db_entries(&app_data_dir, game_type)?;
            attach_user_aliases(
                &mut entries,
                &load_user_aliases(&app.state::<sqlx::SqlitePool>(), game_type).await,
            );
            Ok(deep_matcher::MasterDb::new(entries))
        })
        .await
}

/// Aliases the user typed on their own objects, grouped by matched entry key.
///
/// The UI has always let users add aliases to an object's custom skins, but
/// they were written to the `objects` table and never read back by the matcher,
/// so they changed nothing. This is the read half.
async fn load_user_aliases(
    pool: &sqlx::SqlitePool,
    game_type: i32,
) -> HashMap<String, Vec<String>> {
    let blobs = match crate::modules::catalog::adapters::sqlite::object::get_user_alias_blobs(
        pool, game_type,
    )
    .await
    {
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

/// Fold user aliases into raw entries before matching indexes are constructed.
fn attach_user_aliases(
    entries: &mut [deep_matcher::DbEntry],
    by_entry_key: &HashMap<String, Vec<String>>,
) {
    let mut attached = 0usize;
    for entry in entries {
        let key =
            crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(
                &entry.name,
            );
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
