//! Optional GameBanana provenance enrichment for Discover-originated imports.

use super::types::{ImportItem, MatchEvidence};
use crate::modules::browser::api::BrowserGameBananaProvenance;
use crate::modules::matching::api::GameBananaResult;
use sqlx::SqlitePool;

const GAMEBANANA_METADATA_KEY: &str = "gamebanana";

/// Keep local source analysis authoritative. A missing, stale, or unreachable
/// remote source never prevents analysis or placement.
pub(crate) async fn enrich_source_metadata(
    db: &SqlitePool,
    game_id: &str,
    item: &ImportItem,
) -> serde_json::Value {
    if item.source_metadata.get(GAMEBANANA_METADATA_KEY).is_some() {
        return item.source_metadata.clone();
    }

    let provenance = match crate::modules::browser::api::gamebanana_provenance_for_source_path(
        db,
        game_id,
        &item.source_path,
    )
    .await
    {
        Ok(Some(provenance)) => provenance,
        Ok(None) => return item.source_metadata.clone(),
        Err(error) => {
            log::debug!(
                "GameBanana provenance lookup skipped for '{}': {error}",
                item.source_path
            );
            return item.source_metadata.clone();
        }
    };

    let item_type = provenance.item_type.clone();
    let item_id = provenance.item_id;
    let metadata = match tokio::task::spawn_blocking(move || {
        crate::modules::matching::api::enrich_gamebanana_item(item_type, item_id)
    })
    .await
    {
        Ok(metadata) => metadata,
        Err(error) => {
            log::warn!(
                "GameBanana enrichment worker stopped for '{}': {error}",
                item.source_path
            );
            GameBananaResult::default()
        }
    };

    merge_gamebanana_metadata(&item.source_metadata, &provenance, &metadata)
}

pub(crate) fn source_metadata_evidence(metadata: &serde_json::Value) -> Option<MatchEvidence> {
    let gamebanana = metadata.get(GAMEBANANA_METADATA_KEY)?.as_object()?;
    let item_type = gamebanana.get("itemType")?.as_str()?;
    let item_id = gamebanana.get("itemId")?.as_u64()?;
    let value = gamebanana
        .get("modName")
        .and_then(serde_json::Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("{item_type} #{item_id}: {name}"))
        .unwrap_or_else(|| format!("{item_type} #{item_id}"));

    Some(MatchEvidence {
        source: GAMEBANANA_METADATA_KEY.to_string(),
        value,
        score: 0.0,
    })
}

fn merge_gamebanana_metadata(
    existing: &serde_json::Value,
    provenance: &BrowserGameBananaProvenance,
    metadata: &GameBananaResult,
) -> serde_json::Value {
    let mut merged = existing
        .as_object()
        .cloned()
        .unwrap_or_else(serde_json::Map::new);
    let status = if metadata.mod_name.is_some()
        || metadata.root_category.is_some()
        || !metadata.file_stems.is_empty()
        || !metadata.description_keywords.is_empty()
    {
        "available"
    } else {
        "unavailable"
    };
    merged.insert(
        GAMEBANANA_METADATA_KEY.to_string(),
        serde_json::json!({
            "originUrl": provenance.origin_page_url,
            "itemType": provenance.item_type,
            "itemId": provenance.item_id,
            "modName": metadata.mod_name,
            "rootCategory": metadata.root_category,
            "fileStems": metadata.file_stems,
            "descriptionKeywords": metadata.description_keywords,
            "status": status,
            "fetchedAt": chrono::Utc::now().to_rfc3339(),
        }),
    );
    serde_json::Value::Object(merged)
}

#[cfg(test)]
mod tests {
    use super::{merge_gamebanana_metadata, source_metadata_evidence};
    use crate::modules::browser::api::BrowserGameBananaProvenance;
    use crate::modules::matching::api::GameBananaResult;

    #[test]
    fn preserves_existing_source_metadata_and_records_verified_gamebanana_data() {
        let metadata = merge_gamebanana_metadata(
            &serde_json::json!({"otherSource": {"id": "local"}}),
            &BrowserGameBananaProvenance {
                origin_page_url: "https://gamebanana.com/mods/528562".to_string(),
                item_type: "Mod".to_string(),
                item_id: 528562,
            },
            &GameBananaResult {
                file_stems: vec!["ayaka_skin".to_string()],
                mod_name: Some("Ayaka Skin".to_string()),
                root_category: Some("Skins".to_string()),
                description_keywords: vec!["ayaka".to_string()],
            },
        );

        assert_eq!(metadata["otherSource"]["id"], "local");
        assert_eq!(metadata["gamebanana"]["itemId"], 528562);
        assert_eq!(metadata["gamebanana"]["status"], "available");
        assert_eq!(
            source_metadata_evidence(&metadata)
                .expect("GameBanana evidence")
                .value,
            "Mod #528562: Ayaka Skin"
        );
    }
}
