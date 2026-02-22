use crate::services::scanner::core::normalizer;
use crate::services::scanner::deep_matcher::{MasterDb, MatchStatus, StagedMatchResult};

/// Detect skin/variant for a staged match result.
///
/// Checks the best candidate's object against `custom_skins` in the MasterDb.
/// Returns `(skin_name, skin_folder_name)` if a skin token overlaps the folder name tokens.
///
/// # Covers: Epic 2 §B.3
pub fn detect_skin_for_staged(
    result: &StagedMatchResult,
    folder_name: &str,
    db: &MasterDb,
) -> (Option<String>, Option<String>) {
    let best = match result.status {
        MatchStatus::AutoMatched | MatchStatus::NeedsReview => result
            .best
            .as_ref()
            .or_else(|| result.candidates_topk.first()),
        MatchStatus::NoMatch => None,
    };

    let Some(best_candidate) = best else {
        return (None, None);
    };

    if best_candidate.object_type != "Character" {
        return (None, None);
    }

    let folder_tokens = normalizer::preprocess_text(folder_name);

    let Some(entry) = db.entries.iter().find(|e| e.name == best_candidate.name) else {
        log::trace!(
            "Skin resolver: no DB entry for '{}', skipping",
            best_candidate.name
        );
        return (None, None);
    };

    if entry.custom_skins.is_empty() {
        log::trace!(
            "Skin resolver: '{}' has 0 custom_skins in schema",
            entry.name
        );
    }

    for skin in &entry.custom_skins {
        let name_tokens = normalizer::preprocess_text(&skin.name);
        if !name_tokens.is_disjoint(&folder_tokens) {
            log::debug!("Skin resolved: '{}' -> skin '{}'", folder_name, skin.name);
            return (Some(skin.name.clone()), skin.aliases.first().cloned());
        }
        for alias in &skin.aliases {
            let alias_tokens = normalizer::preprocess_text(alias);
            if !alias_tokens.is_disjoint(&folder_tokens) {
                log::debug!(
                    "Skin resolved via alias: '{}' -> skin '{}' (alias '{}')",
                    folder_name,
                    skin.name,
                    alias
                );
                return (Some(skin.name.clone()), skin.aliases.first().cloned());
            }
        }
    }

    (None, None)
}
