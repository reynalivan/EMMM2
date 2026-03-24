use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ThemeConfig {
    pub colors: std::collections::HashMap<String, String>,
    pub glass: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct CustomTheme {
    pub id: String,
    pub label: String,
    pub config: ThemeConfig,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ThemeMetadata {
    pub id: String,
    pub label: String,
}

fn get_themes_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let themes_dir = app_data_dir.join("themes");

    if !themes_dir.exists() {
        fs::create_dir_all(&themes_dir).map_err(|e| e.to_string())?;
    }

    Ok(themes_dir)
}

#[tauri::command]
#[specta::specta]
pub async fn list_custom_themes(app_handle: AppHandle) -> Result<Vec<ThemeMetadata>, String> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(theme) = serde_json::from_str::<CustomTheme>(&content) {
                        themes.push(ThemeMetadata {
                            id: theme.id,
                            label: theme.label,
                        });
                    }
                }
            }
        }
    }

    Ok(themes)
}

#[tauri::command]
#[specta::specta]
pub async fn load_custom_theme(app_handle: AppHandle, id: String) -> Result<CustomTheme, String> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme_path = themes_dir.join(format!("{}.json", id));

    if !theme_path.exists() {
        return Err(format!("Theme '{}' not found", id));
    }

    let content = fs::read_to_string(theme_path).map_err(|e| e.to_string())?;
    let theme = serde_json::from_str::<CustomTheme>(&content).map_err(|e| e.to_string())?;

    Ok(theme)
}

#[tauri::command]
#[specta::specta]
pub async fn save_custom_theme(app_handle: AppHandle, theme: CustomTheme) -> Result<(), String> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme_path = themes_dir.join(format!("{}.json", theme.id));

    let content = serde_json::to_string_pretty(&theme).map_err(|e| e.to_string())?;
    fs::write(theme_path, content).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_custom_theme(app_handle: AppHandle, id: String) -> Result<(), String> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme_path = themes_dir.join(format!("{}.json", id));

    if theme_path.exists() {
        fs::remove_file(theme_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}
