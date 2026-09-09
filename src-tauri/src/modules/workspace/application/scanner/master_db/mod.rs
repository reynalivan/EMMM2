//! MasterDB loading, matching, and thumbnail path resolution service.
//!
//! Centralises the filesystem read + JSON parse + thumbnail resolution
//! logic for the MasterDB that was previously duplicated across commands.

use crate::shared::errors::ScannerError;

use std::path::Path;

use crate::modules::games::application::game::schema_loader;
use crate::modules::matching::application::deep_matcher::analysis::content::{
    IniTokenizationConfig, PreparedTokenFilters,
};
use crate::modules::matching::application::deep_matcher::{DbEntry, EntryKind, MasterDb};
use serde::Deserialize;

#[derive(Deserialize)]
struct MasterDbPayload {
    entries: Vec<DbEntry>,
}

/// Load and parse the MasterDB JSON for a given game type from `resource_dir`.
pub fn load_master_db_entries(
    resource_dir: &Path,
    game_type: i32,
) -> Result<Vec<DbEntry>, ScannerError> {
    let canonical = schema_loader::normalize_game_type(game_type);
    let db_path = resource_dir
        .join("databases")
        .join(format!("{}.json", canonical));

    if !db_path.exists() {
        log::warn!(
            "MasterDB not found for {}: {}",
            game_type,
            db_path.display()
        );
        return Ok(Vec::new());
    }

    let json_content = std::fs::read_to_string(&db_path)?;

    // Only accept strict object format
    let payload: MasterDbPayload =
        serde_json::from_str(&json_content).map_err(|e| ScannerError::Parse {
            what: "MasterDB".to_string(),
            detail: format!("expected an object with an 'entries' key: {}", e),
        })?;

    let mut entries = payload.entries;
    for entry in entries.iter_mut() {
        // We can reuse the `absolutize_thumbnails` logic by swapping out the entry
        // with an empty dummy, transforming it, and placing it back.
        // A cleaner way since `absolutize_thumbnails` takes ownership:
        let old = std::mem::replace(
            entry,
            DbEntry {
                name: String::new(),
                aliases: vec![],
                object_type: String::new(),
                entry_kind: Default::default(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hash_db: Default::default(),
            },
        );
        *entry = absolutize_thumbnails(old, resource_dir);
    }

    Ok(entries)
}

pub fn ini_filters(resource_dir: Option<&Path>, game_type: i32) -> PreparedTokenFilters {
    let Some(resource_dir) = resource_dir else {
        return IniTokenizationConfig::default().prepare();
    };
    let schema = schema_loader::load_schema(resource_dir, game_type);
    IniTokenizationConfig {
        stopwords: schema.stopwords,
        short_token_whitelist: schema.short_token_whitelist,
        ini_key_blacklist: schema.ini_key_blacklist,
        ini_key_whitelist: schema.ini_key_whitelist,
    }
    .prepare()
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SearchResultEntry {
    pub item: DbEntry,
    pub score: f32,
}

pub fn fuzzy_score(query: &str, target: &str) -> f32 {
    let q = query.to_lowercase();
    let t = target.to_lowercase();

    if q.is_empty() || t.is_empty() {
        return 0.0;
    }

    if t.contains(&q) || q.contains(&t) {
        return 1.0;
    }

    let q_chars: Vec<char> = q.chars().collect();
    let t_chars: Vec<char> = t.chars().collect();
    let m = q_chars.len();
    let n = t_chars.len();

    if m == 0 || n == 0 {
        return 0.0;
    }

    let mut prev = vec![0; n + 1];
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        for j in 1..=n {
            if q_chars[i - 1] == t_chars[j - 1] {
                curr[j] = prev[j - 1] + 1;
            } else {
                curr[j] = std::cmp::max(prev[j], curr[j - 1]);
            }
        }
        prev.copy_from_slice(&curr);
        curr.fill(0);
    }

    let lcs = prev[n] as f32;
    lcs / (std::cmp::min(m, n) as f32)
}

/// Rewrite the entry's resource-relative thumbnail paths to absolute ones for
/// the frontend. Applied only to entries that survive scoring.
fn absolutize_thumbnails(mut entry: DbEntry, resource_dir: &Path) -> DbEntry {
    if let Some(ref thumb) = entry.thumbnail_path {
        if let Some(abs_path) = resource_dir.join(thumb).to_str() {
            entry.thumbnail_path = Some(abs_path.to_string());
        }
    }
    for skin in &mut entry.custom_skins {
        if let Some(ref thumb) = skin.thumbnail_skin_path {
            if let Some(abs_path) = resource_dir.join(thumb).to_str() {
                skin.thumbnail_skin_path = Some(abs_path.to_string());
            }
        }
    }
    entry
}

pub fn search_master_db_service(
    db: &MasterDb,
    resource_dir: &Path,
    query: &str,
    object_type: Option<&str>,
) -> Vec<SearchResultEntry> {
    let query_lower = query.trim().to_lowercase();
    let type_filter = object_type.map(|t| t.to_lowercase());

    let mut results = Vec::new();
    let fuzzy_threshold = 0.2;

    for entry in &db.entries {
        if entry.entry_kind == EntryKind::Taxonomy {
            continue;
        }
        if let Some(ref t) = type_filter {
            if entry.object_type.to_lowercase() != *t {
                continue;
            }
        }

        // Score first, clone second: a `DbEntry` carries tags, metadata,
        // custom_skins and hash_db, and most entries are discarded below.
        if query_lower.is_empty() {
            results.push(SearchResultEntry {
                item: absolutize_thumbnails(entry.clone(), resource_dir),
                score: 1.0,
            });
            continue;
        }

        let mut is_direct_match = entry.name.to_lowercase().contains(&query_lower);
        if !is_direct_match {
            is_direct_match = entry
                .aliases
                .iter()
                .any(|alias| alias.to_lowercase().contains(&query_lower));
        }
        if !is_direct_match {
            is_direct_match = entry.custom_skins.iter().any(|skin| {
                skin.name.to_lowercase().contains(&query_lower)
                    || skin
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase().contains(&query_lower))
            });
        }

        let score = if is_direct_match {
            1.0
        } else if query_lower.len() < 3 {
            0.0
        } else {
            let mut max_score = fuzzy_score(&query_lower, &entry.name);
            for alias in &entry.aliases {
                let alias_score = fuzzy_score(&query_lower, alias);
                if alias_score > max_score {
                    max_score = alias_score;
                }
            }
            for skin in &entry.custom_skins {
                max_score = max_score.max(fuzzy_score(&query_lower, &skin.name));
                for alias in &skin.aliases {
                    max_score = max_score.max(fuzzy_score(&query_lower, alias));
                }
            }
            max_score
        };

        if score >= fuzzy_threshold {
            results.push(SearchResultEntry {
                item: absolutize_thumbnails(entry.clone(), resource_dir),
                score,
            });
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.item.name.cmp(&b.item.name))
    });

    results.into_iter().take(20).collect()
}

mod cache;
pub use cache::{get_cached, MasterDbCache};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
