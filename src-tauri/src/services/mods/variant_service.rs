use crate::repo::mod_repo;
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct VariantGroup {
    pub current_mod_id: String,
    pub current_mod_path: String,
    pub variants: Vec<VariantEntry>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct VariantEntry {
    pub mod_id: String,
    pub folder_path: String,
    pub name: String,
}

/// Discover variants for the currently active mod in a given "scope".
/// In EMMM, a scope is typically an Object (Character).
pub async fn discover_variants(
    pool: &SqlitePool,
    _game_id: &str,
    object_id: &str,
    is_safe: bool,
) -> Result<Option<VariantGroup>, String> {
    let _is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    // 1. Get all mods for this object in this corridor
    let mods = mod_repo::get_mods_by_object_id(pool, object_id, is_safe)
        .await
        .map_err(|e| e.to_string())?;

    if mods.is_empty() {
        return Ok(None);
    }

    // 2. Separate into Enabled and Disabled
    let mut enabled_mod = None;
    let mut variants = Vec::new();

    for m in mods {
        let entry = VariantEntry {
            mod_id: m.id.clone(),
            folder_path: m.folder_path.clone(),
            name: Path::new(&m.folder_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| m.actual_name.clone()),
        };

        if m.status == crate::database::models::ItemStatus::Enabled {
            enabled_mod = Some(entry.clone());
        }
        variants.push(entry);
    }

    // Sort variants by name for consistent cycling
    variants.sort_by(|a, b| a.name.cmp(&b.name));

    if let Some(current) = enabled_mod {
        Ok(Some(VariantGroup {
            current_mod_id: current.mod_id,
            current_mod_path: current.folder_path,
            variants,
        }))
    } else if !variants.is_empty() {
        // No mod enabled, but variants exist -> we can "start" cycling by enabling the first one
        Ok(Some(VariantGroup {
            current_mod_id: String::new(),
            current_mod_path: String::new(),
            variants,
        }))
    } else {
        Ok(None)
    }
}
