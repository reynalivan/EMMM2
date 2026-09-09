//! Read-only folder inspection commands used by workspace tooltips.

use crate::shared::errors::AppError;

#[tauri::command]
#[specta::specta]
pub async fn list_folder_entries_cmd(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_path: String,
    game_id: String,
) -> Result<
    Vec<crate::modules::workspace::application::scanner::folder_entries::FolderEntry>,
    AppError,
> {
    Ok(
        crate::modules::workspace::application::scanner::folder_entries::list_folder_entries(
            pool.inner(),
            &game_id,
            &folder_path,
        )
        .await?,
    )
}
