//! Read-only folder inspection commands used by workspace tooltips.

use crate::domain::errors::AppError;

#[tauri::command]
#[specta::specta]
pub async fn list_folder_entries_cmd(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_path: String,
    game_id: String,
) -> Result<Vec<crate::services::scanner::folder_entries::FolderEntry>, AppError> {
    Ok(
        crate::services::scanner::folder_entries::list_folder_entries(
            pool.inner(),
            &game_id,
            &folder_path,
        )
        .await?,
    )
}
