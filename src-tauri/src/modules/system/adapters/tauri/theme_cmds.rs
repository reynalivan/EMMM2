use crate::platform::fs::atomic_file::atomic_write;
use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

const MAX_THEME_ID_LEN: usize = 64;
const MAX_THEME_LABEL_LEN: usize = 128;
const MAX_THEME_CONFIG_ENTRIES: usize = 128;
const MAX_THEME_CONFIG_KEY_LEN: usize = 64;
const MAX_THEME_CONFIG_VALUE_LEN: usize = 512;
const MAX_THEME_FILE_SIZE: usize = 64 * 1024;

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

fn validation_error(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn validate_slug(value: &str, field: &str, max_len: usize) -> Result<(), AppError> {
    if value.is_empty() || value.len() > max_len || !value.is_ascii() {
        return Err(validation_error(format!(
            "{field} must be 1-{max_len} ASCII characters"
        )));
    }
    let mut bytes = value.bytes();
    if !bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(validation_error(format!(
            "{field} must start with an ASCII letter or digit and contain only letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

fn validate_css_value(value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() || value.len() > MAX_THEME_CONFIG_VALUE_LEN {
        return Err(validation_error(format!(
            "Theme config values must be 1-{MAX_THEME_CONFIG_VALUE_LEN} characters"
        )));
    }
    if value
        .chars()
        .any(|character| character.is_control() || matches!(character, ';' | '{' | '}' | '<' | '>'))
    {
        return Err(validation_error(
            "Theme config values cannot contain CSS rule delimiters or control characters",
        ));
    }
    let lowercase = value.to_ascii_lowercase();
    if lowercase.contains("url(")
        || lowercase.contains("@import")
        || lowercase.contains("expression(")
    {
        return Err(validation_error(
            "Theme config values cannot load external resources or execute expressions",
        ));
    }
    Ok(())
}

fn validate_config_map(
    values: &std::collections::HashMap<String, String>,
    field: &str,
) -> Result<(), AppError> {
    for (key, value) in values {
        validate_slug(key, &format!("Theme {field} key"), MAX_THEME_CONFIG_KEY_LEN)?;
        validate_css_value(value)?;
    }
    Ok(())
}

fn validate_theme(theme: &CustomTheme) -> Result<(), AppError> {
    validate_slug(&theme.id, "Theme ID", MAX_THEME_ID_LEN)?;
    if theme.label.trim().is_empty()
        || theme.label.len() > MAX_THEME_LABEL_LEN
        || theme.label.chars().any(char::is_control)
    {
        return Err(validation_error(format!(
            "Theme label must be 1-{MAX_THEME_LABEL_LEN} characters without control characters"
        )));
    }
    if theme.config.colors.len() + theme.config.glass.len() > MAX_THEME_CONFIG_ENTRIES {
        return Err(validation_error(format!(
            "Theme config cannot contain more than {MAX_THEME_CONFIG_ENTRIES} entries"
        )));
    }
    validate_config_map(&theme.config.colors, "color")?;
    validate_config_map(&theme.config.glass, "glass")?;
    Ok(())
}

fn theme_path(themes_dir: &Path, id: &str) -> Result<PathBuf, AppError> {
    validate_slug(id, "Theme ID", MAX_THEME_ID_LEN)?;
    Ok(themes_dir.join(format!("{id}.json")))
}

fn parse_theme_bytes(content: &[u8]) -> Result<CustomTheme, AppError> {
    if content.len() > MAX_THEME_FILE_SIZE {
        return Err(validation_error(format!(
            "Theme file exceeds the {MAX_THEME_FILE_SIZE}-byte limit"
        )));
    }
    let theme = serde_json::from_slice::<CustomTheme>(content)
        .map_err(|error| validation_error(format!("Invalid theme JSON: {error}")))?;
    validate_theme(&theme)?;
    Ok(theme)
}

fn read_theme_file(path: &Path) -> Result<CustomTheme, AppError> {
    let mut content = Vec::new();
    File::open(path)?
        .take((MAX_THEME_FILE_SIZE + 1) as u64)
        .read_to_end(&mut content)?;
    parse_theme_bytes(&content)
}

fn serialize_theme(theme: &CustomTheme) -> Result<Vec<u8>, AppError> {
    validate_theme(theme)?;
    let content = serde_json::to_vec_pretty(theme)
        .map_err(|error| validation_error(format!("Invalid theme data: {error}")))?;
    if content.len() > MAX_THEME_FILE_SIZE {
        return Err(validation_error(format!(
            "Theme file exceeds the {MAX_THEME_FILE_SIZE}-byte limit"
        )));
    }
    Ok(content)
}

fn save_theme_to_dir(themes_dir: &Path, theme: &CustomTheme) -> Result<(), AppError> {
    fs::create_dir_all(themes_dir)?;
    let path = theme_path(themes_dir, &theme.id)?;
    atomic_write(&path, &serialize_theme(theme)?)
}

fn load_theme_from_dir(themes_dir: &Path, id: &str) -> Result<CustomTheme, AppError> {
    let path = theme_path(themes_dir, id)?;
    if !path.exists() {
        return Err(AppError::NotFound(format!("Theme '{id}' not found")));
    }
    let theme = read_theme_file(&path)?;
    if theme.id != id {
        return Err(validation_error(format!(
            "Theme file ID '{}' does not match requested ID '{id}'",
            theme.id
        )));
    }
    Ok(theme)
}

fn delete_theme_from_dir(themes_dir: &Path, id: &str) -> Result<(), AppError> {
    let path = theme_path(themes_dir, id)?;
    if path.exists() {
        crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&path)?;
    }
    Ok(())
}

fn get_themes_dir(app_handle: &AppHandle) -> Result<PathBuf, AppError> {
    let app_data_dir = app_handle.path().app_data_dir()?;
    let themes_dir = app_data_dir.join("themes");

    if !themes_dir.exists() {
        fs::create_dir_all(&themes_dir)?;
    }

    Ok(themes_dir)
}

#[tauri::command]
#[specta::specta]
pub async fn list_custom_themes(app_handle: AppHandle) -> Result<Vec<ThemeMetadata>, AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = path.file_stem().and_then(|stem| stem.to_str()).filter(|_| {
                path.extension().and_then(|extension| extension.to_str()) == Some("json")
            }) else {
                continue;
            };
            if let Ok(theme) = load_theme_from_dir(&themes_dir, id) {
                themes.push(ThemeMetadata {
                    id: theme.id,
                    label: theme.label,
                });
            }
        }
    }

    Ok(themes)
}

#[tauri::command]
#[specta::specta]
pub async fn load_custom_theme(app_handle: AppHandle, id: String) -> Result<CustomTheme, AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    load_theme_from_dir(&themes_dir, &id)
}

#[tauri::command]
#[specta::specta]
pub async fn save_custom_theme(app_handle: AppHandle, theme: CustomTheme) -> Result<(), AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    save_theme_to_dir(&themes_dir, &theme)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_custom_theme(app_handle: AppHandle, id: String) -> Result<(), AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    delete_theme_from_dir(&themes_dir, &id)
}

#[tauri::command]
#[specta::specta]
pub async fn import_custom_theme(app_handle: AppHandle) -> Result<Option<CustomTheme>, AppError> {
    let selected = app_handle
        .dialog()
        .file()
        .add_filter("JSON Theme", &["json"])
        .blocking_pick_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|_| {
        validation_error("Selected theme is not available as a local filesystem path")
    })?;
    let theme = read_theme_file(&path)?;
    let themes_dir = get_themes_dir(&app_handle)?;
    save_theme_to_dir(&themes_dir, &theme)?;
    Ok(Some(theme))
}

#[tauri::command]
#[specta::specta]
pub async fn export_custom_theme(
    app_handle: AppHandle,
    id: String,
) -> Result<Option<String>, AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme = load_theme_from_dir(&themes_dir, &id)?;
    let content = serialize_theme(&theme)?;
    let selected = app_handle
        .dialog()
        .file()
        .add_filter("JSON Theme", &["json"])
        .set_file_name(format!("{}.json", theme.id))
        .blocking_save_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|_| {
        validation_error("Selected export destination is not a local filesystem path")
    })?;
    atomic_write(&path, &content)?;

    Ok(Some(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("theme.json")
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn valid_theme() -> CustomTheme {
        CustomTheme {
            id: "ocean-night_2".to_string(),
            label: "Ocean Night".to_string(),
            config: ThemeConfig {
                colors: HashMap::from([
                    ("primary".to_string(), "oklch(65% 0.2 250)".to_string()),
                    ("base-100".to_string(), "#101827".to_string()),
                ]),
                glass: HashMap::from([(
                    "bg".to_string(),
                    "color-mix(in srgb, var(--color-base-100) 40%, transparent)".to_string(),
                )]),
            },
        }
    }

    #[test]
    fn rejects_unsafe_theme_ids_and_path_traversal() {
        let themes_dir = tempdir().expect("create themes directory");
        for id in [
            "",
            ".",
            "..",
            "../outside",
            "theme/name",
            "theme\\name",
            "-leading",
            "contains space",
            "non-ascii-\u{e9}",
        ] {
            assert!(
                theme_path(themes_dir.path(), id).is_err(),
                "accepted {id:?}"
            );
        }

        assert!(theme_path(themes_dir.path(), &"a".repeat(MAX_THEME_ID_LEN + 1)).is_err());
        assert_eq!(
            theme_path(themes_dir.path(), "safe-theme_2").expect("safe theme path"),
            themes_dir.path().join("safe-theme_2.json")
        );
    }

    #[test]
    fn validates_labels_and_config_bounds() {
        let mut theme = valid_theme();
        theme.label = "   ".to_string();
        assert!(validate_theme(&theme).is_err());

        theme = valid_theme();
        theme.label = "a".repeat(MAX_THEME_LABEL_LEN + 1);
        assert!(validate_theme(&theme).is_err());

        theme = valid_theme();
        theme.config.colors = (0..=MAX_THEME_CONFIG_ENTRIES)
            .map(|index| (format!("color-{index}"), "#fff".to_string()))
            .collect();
        assert!(validate_theme(&theme).is_err());
    }

    #[test]
    fn rejects_css_breakout_without_rejecting_existing_value_shapes() {
        validate_theme(&valid_theme()).expect("existing CSS color shapes remain valid");

        let mut unsafe_key = valid_theme();
        unsafe_key
            .config
            .colors
            .insert("primary;}body".to_string(), "red".to_string());
        assert!(validate_theme(&unsafe_key).is_err());

        let mut unsafe_value = valid_theme();
        unsafe_value.config.colors.insert(
            "primary".to_string(),
            "red; } body { display: none".to_string(),
        );
        assert!(validate_theme(&unsafe_value).is_err());
    }

    #[test]
    fn rejects_oversized_theme_files_before_parsing() {
        let oversized = vec![b' '; MAX_THEME_FILE_SIZE + 1];
        assert!(parse_theme_bytes(&oversized).is_err());
    }

    #[test]
    fn canonical_save_and_load_round_trip_and_validate_ids() {
        let themes_dir = tempdir().expect("create themes directory");
        let theme = valid_theme();

        save_theme_to_dir(themes_dir.path(), &theme).expect("save valid theme");
        let loaded = load_theme_from_dir(themes_dir.path(), &theme.id).expect("load saved theme");
        assert_eq!(loaded.id, theme.id);
        assert_eq!(loaded.label, theme.label);
        assert_eq!(loaded.config.colors, theme.config.colors);
        assert!(load_theme_from_dir(themes_dir.path(), "../outside").is_err());
        assert!(delete_theme_from_dir(themes_dir.path(), "../outside").is_err());
    }
}
