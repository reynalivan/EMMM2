//! Deep-matching an extracted archive against the game's Master DB.

use std::path::Path;
use tauri::{AppHandle, Manager};

use crate::repo::browser_repo::ImportJobMatch as MatchResult;
use crate::services::scanner::core::walker::ModCandidate;
use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::{match_folder_phased, MasterDb};

use super::pipeline::count_ini_files;

/// Attempt deep match. If the scanner service has a `quick_folder_match` function, call it.
/// Falls back to confidence 0.0 (needs_review) if the scanner is unavailable.
pub(super) async fn try_deep_match(
    app: &AppHandle,
    extract_dir: &Path,
    game_id: Option<&str>,
) -> Option<MatchResult> {
    // Load all .ini files for basic heuristic
    let ini_count = count_ini_files(extract_dir);

    // Heuristic: if no .ini we already blocked above; return low confidence for manual review
    if ini_count == 0 {
        return None;
    }

    let game_id_str = game_id?;

    // Get the sqlite pool
    let pool = app.state::<sqlx::SqlitePool>();

    // Determine the game_type for the given game_id to load the correct MasterDb schema
    let game_type_res: Option<String> =
        crate::repo::game_repo::get_game_type_raw(pool.inner(), game_id_str)
            .await
            .ok()
            .flatten();

    let game_type = game_type_res?;

    let resource_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("resources")
        .join("databases")
        .join(format!("{}.json", game_type.to_lowercase()));

    if !resource_path.exists() {
        return Some(MatchResult {
            category: None,
            entry_key: None,
            alias_name: None,
            confidence: 0.0,
            reason: Some(format!(
                "Master DB not found at: {}",
                resource_path.display()
            )),
        });
    }

    let json_content = std::fs::read_to_string(&resource_path).ok()?;
    let master_db = MasterDb::from_json(&json_content).ok()?;

    // Build candidate from extract_dir
    let raw_name = extract_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let display_name = crate::common::normalizer::normalize_display_name(&raw_name);
    let candidate = ModCandidate {
        path: extract_dir.to_path_buf(),
        raw_name: raw_name.clone(),
        display_name,
        is_disabled: crate::common::normalizer::is_disabled_folder(&raw_name),
    };

    let content = crate::services::scanner::core::walker::scan_folder_content(extract_dir, 3);
    let ini_config = IniTokenizationConfig::default();
    let ai_config = AiRerankConfig::default();

    let result = match_folder_phased(&candidate, &master_db, &content, &ini_config, &ai_config);

    let top = match result.best {
        Some(c) => c,
        None => {
            return Some(MatchResult {
                category: None,
                entry_key: None,
                alias_name: None,
                confidence: 0.0,
                reason: Some("No match found by deep matcher.".to_string()),
            })
        }
    };

    // Extract numeric confidence for sorting
    let conf_val = match top.confidence {
        crate::services::scanner::deep_matcher::models::types::Confidence::Excellent => 1.0,
        crate::services::scanner::deep_matcher::models::types::Confidence::High => 0.8,
        crate::services::scanner::deep_matcher::models::types::Confidence::Medium => 0.5,
        crate::services::scanner::deep_matcher::models::types::Confidence::Low => 0.2,
        crate::services::scanner::deep_matcher::models::types::Confidence::None => 0.0,
    };

    Some(MatchResult {
        category: Some(top.object_type),
        entry_key: Some(crate::services::scanner::sync::helpers::canonical_entry_key(&top.name)),
        alias_name: Some(top.name),
        confidence: conf_val,
        reason: top.reasons.first().map(|r| format!("{:?}", r)),
    })
}
