//! MasterDB loading, matching, and thumbnail path resolution service.
//!
//! Centralises the filesystem read + JSON parse + thumbnail resolution
//! logic for the MasterDB that was previously duplicated across commands.

use serde_json::Value;
use std::path::Path;

use crate::services::game::schema_loader;
use crate::services::scanner::core::types;
use crate::services::scanner::core::walker::{FolderContent, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::{self, DbEntry, MasterDb, StagedMatchResult};

/// Load and parse the MasterDB JSON for a given game type from `resource_dir`.
pub fn load_master_db_json(resource_dir: &Path, game_type: &str) -> Result<String, String> {
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
        return Ok("[]".to_string());
    }

    let json_content =
        std::fs::read_to_string(&db_path).map_err(|e| format!("Failed to read MasterDB: {e}"))?;

    let parsed: Value = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse MasterDB JSON: {e}"))?;

    let mut entries: Vec<Value> = match parsed {
        Value::Object(ref map) if map.contains_key("entries") => {
            serde_json::from_value(map["entries"].clone())
                .map_err(|e| format!("Failed to parse entries: {e}"))?
        }
        Value::Array(arr) => arr,
        _ => {
            return Err(
                "Invalid MasterDB format: expected array or object with 'entries' key".to_string(),
            )
        }
    };

    resolve_entry_thumbnails(&mut entries, resource_dir);
    serde_json::to_string(&entries).map_err(|e| format!("Failed to serialize MasterDB: {e}"))
}

/// Resolve all thumbnail fields in a slice of serde_json entries to absolute paths.
pub fn resolve_entry_thumbnails(entries: &mut Vec<Value>, resource_dir: &Path) {
    for entry in entries.iter_mut() {
        if let Some(thumb_rel) = entry.get("thumbnail_path").and_then(|v| v.as_str()) {
            let abs_path = resource_dir.join(thumb_rel);
            if let Some(abs_str) = abs_path.to_str() {
                entry["thumbnail_path"] = Value::String(abs_str.to_string());
            }
        }

        if let Some(skins) = entry.get_mut("custom_skins").and_then(|v| v.as_array_mut()) {
            for skin in skins {
                if let Some(skin_thumb_rel) =
                    skin.get("thumbnail_skin_path").and_then(|v| v.as_str())
                {
                    let abs_path = resource_dir.join(skin_thumb_rel);
                    if let Some(abs_str) = abs_path.to_str() {
                        skin["thumbnail_skin_path"] = Value::String(abs_str.to_string());
                    }
                }
            }
        }
    }
}

/// Matched DB entry returned to frontend with resolved absolute thumbnail path.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MatchedDbEntry {
    pub name: String,
    pub object_type: String,
    pub tags: Vec<String>,
    pub metadata: Option<Value>,
    pub thumbnail_path: Option<String>,
    pub match_level: String,
    pub match_confidence: String,
    pub match_detail: String,
}

pub fn match_object_with_staged_pipeline(db: &MasterDb, object_name: &str) -> StagedMatchResult {
    let candidate = ModCandidate {
        path: std::path::PathBuf::from(object_name),
        raw_name: object_name.to_string(),
        display_name: object_name.to_string(),
        is_disabled: false,
    };
    let content = FolderContent {
        subfolder_names: Vec::new(),
        files: Vec::new(),
        ini_files: Vec::new(),
    };
    let ini_config = IniTokenizationConfig::default();

    deep_matcher::match_folder_phased(
        &candidate,
        db,
        &content,
        &ini_config,
        &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig::default(),
    )
}

pub fn resolve_thumbnail_path(resource_dir: &Path, entry: &DbEntry) -> Option<String> {
    let rel_path = entry.thumbnail_path.as_ref()?;
    let abs_path = resource_dir.join(rel_path);

    if !abs_path.exists() {
        log::warn!(
            "Thumbnail not found for {}: {}",
            entry.name,
            abs_path.display()
        );
        return None;
    }

    abs_path.to_str().map(|path| path.to_string())
}

pub fn build_matched_db_entry_from_staged(
    resource_dir: &Path,
    db: &MasterDb,
    match_result: &StagedMatchResult,
) -> Option<MatchedDbEntry> {
    let candidate = types::staged_primary_candidate(match_result)?;
    let entry = db.entries.get(candidate.entry_id)?;

    Some(MatchedDbEntry {
        name: entry.name.clone(),
        object_type: entry.object_type.clone(),
        tags: entry.tags.clone(),
        metadata: entry.metadata.clone(),
        thumbnail_path: resolve_thumbnail_path(resource_dir, entry),
        match_level: types::match_status_label(&match_result.status).to_string(),
        match_confidence: types::staged_confidence_label(match_result).to_string(),
        match_detail: types::staged_match_detail(match_result),
    })
}

pub fn match_object_with_db_service(
    resource_dir: &Path,
    game_type: &str,
    object_name: &str,
) -> Result<Option<MatchedDbEntry>, String> {
    let canonical = schema_loader::normalize_game_type(game_type);
    let db_path = resource_dir
        .join("databases")
        .join(format!("{}.json", canonical));

    if !db_path.exists() {
        return Ok(None);
    }

    let json =
        std::fs::read_to_string(&db_path).map_err(|e| format!("Failed to read MasterDB: {e}"))?;

    let db = MasterDb::from_json(&json)?;

    let match_result = match_object_with_staged_pipeline(&db, object_name);
    Ok(build_matched_db_entry_from_staged(
        resource_dir,
        &db,
        &match_result,
    ))
}

#[derive(Debug, serde::Serialize)]
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
        if let Some(ref t) = type_filter {
            if query_lower.is_empty() && entry.object_type.to_lowercase() != *t {
                continue;
            }
        }

        let mut entry_clone = entry.clone();

        if let Some(ref thumb) = entry_clone.thumbnail_path {
            if let Some(abs_path) = resource_dir.join(thumb).to_str() {
                entry_clone.thumbnail_path = Some(abs_path.to_string());
            }
        }
        for skin in &mut entry_clone.custom_skins {
            if let Some(ref thumb) = skin.thumbnail_skin_path {
                if let Some(abs_path) = resource_dir.join(thumb).to_str() {
                    skin.thumbnail_skin_path = Some(abs_path.to_string());
                }
            }
        }

        if query_lower.is_empty() {
            results.push(SearchResultEntry {
                item: entry_clone,
                score: 1.0,
            });
            continue;
        }

        let mut is_direct_match = entry.name.to_lowercase().contains(&query_lower);
        if !is_direct_match {
            is_direct_match = entry
                .tags
                .iter()
                .any(|alias| alias.to_lowercase().contains(&query_lower));
        }

        let score = if is_direct_match {
            1.0
        } else if query_lower.len() < 3 {
            0.0
        } else {
            let mut max_score = fuzzy_score(&query_lower, &entry.name);
            for alias in &entry.tags {
                let alias_score = fuzzy_score(&query_lower, alias);
                if alias_score > max_score {
                    max_score = alias_score;
                }
            }
            max_score
        };

        if score >= fuzzy_threshold {
            results.push(SearchResultEntry {
                item: entry_clone,
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
