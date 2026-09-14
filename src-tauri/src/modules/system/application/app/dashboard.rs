use crate::modules::library::application::ini::document::{
    list_ini_files, read_ini_document, IniDocument, KeyBinding,
};
use crate::shared::errors::AppError;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ActiveKeyControlKind {
    KeyBinding,
    KeyToggle,
}

/// A keybinding entry extracted from an enabled mod's INI file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ActiveKeyBinding {
    pub mod_name: String,
    pub folder_path: String,
    pub section_name: String,
    pub key: Option<String>,
    pub back: Option<String>,
    pub control_kind: ActiveKeyControlKind,
    pub value_summary: Option<String>,
}

struct ActiveKeyBindingDetails {
    key_binding: KeyBinding,
    control_kind: ActiveKeyControlKind,
    value_summary: Option<String>,
}

pub async fn get_active_keybindings_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<Vec<ActiveKeyBinding>, AppError> {
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} has no mods path")))?;
    let mods_root = std::path::PathBuf::from(mods_root);
    // 1. Fetch enabled mods' folder paths and names for this game
    let rows = crate::modules::library::adapters::sqlite::mods::get_enabled_mods_names_and_paths(
        pool, game_id,
    )
    .await?;

    tokio::task::spawn_blocking(move || harvest_active_keybindings_for_mods(&mods_root, rows))
        .await?
}

fn harvest_active_keybindings_for_mods(
    mods_root: &std::path::Path,
    rows: Vec<(
        String,
        crate::modules::system::domain::mod_path::ModFolderPath,
    )>,
) -> Result<Vec<ActiveKeyBinding>, AppError> {
    let mut bindings = Vec::new();
    for (mod_name, folder_path) in rows {
        let folder_path = folder_path.resolve(mods_root);
        let Ok(keybinds) = harvest_active_keybindings(&folder_path) else {
            continue;
        };
        bindings.extend(
            keybinds
                .into_iter()
                .filter(|binding| {
                    binding.key_binding.key.is_some() || binding.key_binding.back.is_some()
                })
                .map(|binding| ActiveKeyBinding {
                    mod_name: mod_name.clone(),
                    folder_path: folder_path.to_string_lossy().to_string(),
                    section_name: binding.key_binding.section_name,
                    key: binding.key_binding.key,
                    back: binding.key_binding.back,
                    control_kind: binding.control_kind,
                    value_summary: binding.value_summary,
                }),
        );
    }
    Ok(bindings)
}

fn harvest_active_keybindings(
    mod_path: &std::path::Path,
) -> Result<Vec<ActiveKeyBindingDetails>, AppError> {
    let mut bindings = Vec::new();
    for ini_path in list_ini_files(mod_path)? {
        let Ok(document) = read_ini_document(&ini_path) else {
            continue;
        };
        let descriptions = describe_key_controls(&document);
        for key_binding in &document.key_bindings {
            let (control_kind, value_summary) = descriptions
                .get(&section_key(&key_binding.section_name))
                .cloned()
                .unwrap_or((ActiveKeyControlKind::KeyBinding, None));
            bindings.push(ActiveKeyBindingDetails {
                key_binding: key_binding.clone(),
                control_kind,
                value_summary,
            });
        }
    }
    Ok(bindings)
}

fn section_key(section_name: &str) -> String {
    section_name.trim().to_ascii_lowercase()
}

fn describe_key_controls(
    document: &IniDocument,
) -> HashMap<String, (ActiveKeyControlKind, Option<String>)> {
    #[derive(Default)]
    struct SectionControl {
        cycle: bool,
        values: Option<Vec<String>>,
    }

    let mut controls = document
        .key_bindings
        .iter()
        .map(|binding| {
            (
                section_key(&binding.section_name),
                SectionControl::default(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut active_section: Option<String> = None;

    for line in &document.raw_lines {
        let trimmed = line.split([';', '#']).next().unwrap_or_default().trim();
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            let key = section_key(header);
            active_section = controls.contains_key(&key).then_some(key);
            continue;
        }
        let Some(section) = active_section.as_ref() else {
            continue;
        };
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let control = controls
            .get_mut(section)
            .expect("active section is tracked by the controls map");
        if key.eq_ignore_ascii_case("type") && value.eq_ignore_ascii_case("cycle") {
            control.cycle = true;
        }
        if key.starts_with('$') {
            let parsed = value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if parsed.len() >= 2 {
                control.values = Some(parsed);
            }
        }
    }

    controls
        .into_iter()
        .map(|(section, control)| {
            let kind = if control.cycle {
                ActiveKeyControlKind::KeyToggle
            } else {
                ActiveKeyControlKind::KeyBinding
            };
            let values = control
                .cycle
                .then(|| control.values.map(|entries| entries.join(", ")))
                .flatten();
            (section, (kind, values))
        })
        .collect()
}

/// Full dashboard payload struct (mirrors the command type).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DashboardPayload {
    pub stats: crate::modules::dashboard::domain::dashboard::DashboardStats,
    #[specta(type = f64)]
    pub duplicate_waste_bytes: i64,
    pub category_distribution: Vec<crate::modules::dashboard::domain::dashboard::CategorySlice>,
    pub game_distribution: Vec<crate::modules::dashboard::domain::dashboard::GameSlice>,
    pub recent_mods: Vec<crate::modules::dashboard::domain::dashboard::RecentMod>,
}

/// Fetch all dashboard data in a single service call.
pub async fn get_dashboard_payload(pool: &sqlx::SqlitePool) -> Result<DashboardPayload, AppError> {
    use crate::modules::dashboard::adapters::sqlite::dashboard;

    let stats = dashboard::fetch_global_stats(pool).await?;

    // Independent reads. Serially they cost four extra round trips; WAL
    // readers do not block each other, so the pool can serve them at once.
    let (duplicate_waste_bytes, category_distribution, game_distribution, recent_mods) = tokio::try_join!(
        async { dashboard::fetch_duplicate_waste(pool).await },
        async { dashboard::fetch_category_distribution(pool).await },
        async { dashboard::fetch_game_distribution(pool).await },
        async { dashboard::fetch_recent_mods(pool, 5).await },
    )?;

    Ok(DashboardPayload {
        stats,
        duplicate_waste_bytes,
        category_distribution,
        game_distribution,
        recent_mods,
    })
}
