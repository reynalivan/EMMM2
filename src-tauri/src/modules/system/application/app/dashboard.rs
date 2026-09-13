use crate::modules::library::application::ini::document::{
    list_ini_files, read_ini_document, IniDocument, KeyBinding,
};
use crate::shared::errors::AppError;

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
    let mods_root = std::path::Path::new(&mods_root);
    // 1. Fetch enabled mods' folder paths and names for this game
    let rows = crate::modules::library::adapters::sqlite::mods::get_enabled_mods_names_and_paths(
        pool, game_id,
    )
    .await?;

    let mut bindings: Vec<ActiveKeyBinding> = Vec::new();

    // 2. For each enabled mod, scan its INI files for keybindings
    for (mod_name, folder_path) in &rows {
        let folder_path = folder_path.resolve(mods_root);
        let Ok(keybinds) = harvest_active_keybindings(&folder_path) else {
            continue;
        };

        let named = keybinds
            .into_iter()
            .filter(|binding| {
                binding.key_binding.key.is_some() || binding.key_binding.back.is_some()
            })
            .map(|kb| ActiveKeyBinding {
                mod_name: mod_name.clone(),
                folder_path: folder_path.to_string_lossy().to_string(),
                section_name: kb.key_binding.section_name,
                key: kb.key_binding.key,
                back: kb.key_binding.back,
                control_kind: kb.control_kind,
                value_summary: kb.value_summary,
            });
        bindings.extend(named);
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
        for key_binding in &document.key_bindings {
            let (control_kind, value_summary) =
                describe_key_control(&document, &key_binding.section_name);
            bindings.push(ActiveKeyBindingDetails {
                key_binding: key_binding.clone(),
                control_kind,
                value_summary,
            });
        }
    }
    Ok(bindings)
}

fn describe_key_control(
    document: &IniDocument,
    section_name: &str,
) -> (ActiveKeyControlKind, Option<String>) {
    let mut in_section = false;
    let mut cycle = false;
    let mut values: Option<Vec<String>> = None;

    for line in &document.raw_lines {
        let trimmed = line.split([';', '#']).next().unwrap_or_default().trim();
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            if in_section {
                break;
            }
            in_section = header.trim().eq_ignore_ascii_case(section_name);
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.eq_ignore_ascii_case("type") && value.eq_ignore_ascii_case("cycle") {
            cycle = true;
        }
        if key.starts_with('$') {
            let parsed = value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if parsed.len() >= 2 {
                values = Some(parsed);
            }
        }
    }

    if cycle {
        return (
            ActiveKeyControlKind::KeyToggle,
            values.map(|entries| entries.join(", ")),
        );
    }
    (ActiveKeyControlKind::KeyBinding, None)
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
