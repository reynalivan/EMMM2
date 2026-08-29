use std::collections::BTreeMap;

use crate::domain::errors::AppError;
use crate::services::scanner::deep_matcher::CustomSkin;

const STABLE_CATEGORIES: [&str; 4] = ["Character", "Weapon", "UI", "Other"];

#[derive(Clone, Debug)]
pub struct CanonicalClassificationMatch {
    pub entry_key: String,
    pub alias_name: Option<String>,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct ObjectClassificationInput {
    pub game_id: String,
    pub object_id: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub metadata: serde_json::Value,
    pub canonical_match: Option<CanonicalClassificationMatch>,
    pub confirmed_source_alias: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClassificationWriteResult {
    pub object_id: String,
    pub child_mods_updated: u64,
    /// The caller uses this to invalidate the parsed MasterDB cache after commit.
    pub aliases_changed: bool,
}

pub async fn apply_object_classification(
    pool: &sqlx::SqlitePool,
    input: ObjectClassificationInput,
) -> Result<ClassificationWriteResult, AppError> {
    let mut tx = pool.begin().await?;
    let result = apply_object_classification_tx(&mut tx, input).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn apply_object_classification_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    input: ObjectClassificationInput,
) -> Result<ClassificationWriteResult, AppError> {
    let category = input.category.trim();
    if !STABLE_CATEGORIES.contains(&category) {
        return Err(AppError::Validation(
            "Category must be Character, Weapon, UI, or Other".to_string(),
        ));
    }
    if input.game_id.trim().is_empty() || input.object_id.trim().is_empty() {
        return Err(AppError::Validation(
            "Game id and object id are required".to_string(),
        ));
    }
    if !input.metadata.is_object() {
        return Err(AppError::Validation(
            "Classification metadata must be a JSON object".to_string(),
        ));
    }

    let canonical_match = validate_canonical_match(input.canonical_match.as_ref())?;
    let source_alias = input
        .confirmed_source_alias
        .as_deref()
        .map(str::trim)
        .filter(|alias| !alias.is_empty());
    if source_alias.is_some() && canonical_match.is_none() {
        return Err(AppError::Validation(
            "A canonical match is required before learning a source alias".to_string(),
        ));
    }

    let subcategory = input
        .subcategory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let metadata_json = serde_json::to_string(&input.metadata)?;
    let current = crate::repo::object::get_classification_object_state_tx(
        tx,
        input.game_id.trim(),
        input.object_id.trim(),
    )
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "Object '{}' was not found for game '{}'",
            input.object_id.trim(),
            input.game_id.trim()
        ))
    })?;

    let (custom_skins_json, aliases_changed) = match source_alias {
        Some(alias) => {
            let mut skins = parse_custom_skins(current.custom_skins.as_deref())?;
            if merge_user_alias(&mut skins, alias) {
                (Some(serde_json::to_string(&skins)?), true)
            } else {
                (None, false)
            }
        }
        None => (None, false),
    };

    let object_updated = crate::repo::object::apply_classification_fields_tx(
        tx,
        input.game_id.trim(),
        input.object_id.trim(),
        crate::repo::object::ClassificationFields {
            category,
            subcategory,
            metadata_json: &metadata_json,
            custom_skins_json: custom_skins_json.as_deref(),
        },
    )
    .await?;
    if object_updated == 0 {
        return Err(AppError::NotFound(format!(
            "Object '{}' was not found for game '{}'",
            input.object_id.trim(),
            input.game_id.trim()
        )));
    }

    crate::repo::object::apply_canonical_match(
        &mut **tx,
        input.object_id.trim(),
        canonical_match.map(|matched| matched.entry_key.trim()),
        canonical_match.and_then(|matched| non_empty(matched.alias_name.as_deref())),
        canonical_match.and_then(|matched| matched.confidence),
        canonical_match.and_then(|matched| non_empty(matched.reason.as_deref())),
        canonical_match.map(|matched| matched.source.trim()),
    )
    .await?;

    let child_mods_updated = crate::repo::mods::set_object_type_for_object(
        &mut **tx,
        input.game_id.trim(),
        input.object_id.trim(),
        category,
    )
    .await?;
    crate::repo::runtime_projection::refresh_projection_for_object_ids_tx(
        tx,
        input.game_id.trim(),
        [input.object_id.trim().to_string()],
    )
    .await?;
    Ok(ClassificationWriteResult {
        object_id: input.object_id.trim().to_string(),
        child_mods_updated,
        aliases_changed,
    })
}

fn validate_canonical_match(
    matched: Option<&CanonicalClassificationMatch>,
) -> Result<Option<&CanonicalClassificationMatch>, AppError> {
    let Some(matched) = matched else {
        return Ok(None);
    };
    if matched.entry_key.trim().is_empty() || matched.source.trim().is_empty() {
        return Err(AppError::Validation(
            "Canonical match entry key and source are required".to_string(),
        ));
    }
    if matched
        .confidence
        .is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
    {
        return Err(AppError::Validation(
            "Canonical match confidence must be between 0 and 1".to_string(),
        ));
    }
    Ok(Some(matched))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn parse_custom_skins(raw: Option<&str>) -> Result<Vec<CustomSkin>, AppError> {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return Ok(Vec::new());
    };
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| {
        AppError::Validation(format!("Stored custom skins are invalid JSON: {error}"))
    })?;
    match value {
        serde_json::Value::Array(_) => serde_json::from_value(value).map_err(|error| {
            AppError::Validation(format!(
                "Stored custom skins have an invalid array shape: {error}"
            ))
        }),
        serde_json::Value::Object(_) => {
            let legacy: BTreeMap<String, String> =
                serde_json::from_value(value).map_err(|error| {
                    AppError::Validation(format!(
                        "Stored custom skins have an invalid legacy shape: {error}"
                    ))
                })?;
            let aliases = legacy
                .into_keys()
                .map(|alias| alias.trim().to_string())
                .filter(|alias| !alias.is_empty())
                .collect::<Vec<_>>();
            if aliases.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![CustomSkin {
                    name: "Legacy".to_string(),
                    aliases,
                    thumbnail_skin_path: None,
                    rarity: None,
                }])
            }
        }
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Err(AppError::Validation(
            "Stored custom skins must be an array or legacy object".to_string(),
        )),
    }
}

fn merge_user_alias(skins: &mut Vec<CustomSkin>, alias: &str) -> bool {
    let alias = alias.trim();
    if skins
        .iter()
        .flat_map(|skin| &skin.aliases)
        .any(|known| known.trim().eq_ignore_ascii_case(alias))
    {
        return false;
    }

    if let Some(user) = skins
        .iter_mut()
        .find(|skin| skin.name.trim().eq_ignore_ascii_case("User"))
    {
        user.name = "User".to_string();
        user.aliases.push(alias.to_string());
    } else {
        skins.push(CustomSkin {
            name: "User".to_string(),
            aliases: vec![alias.to_string()],
            thumbnail_skin_path: None,
            rarity: None,
        });
    }
    true
}
