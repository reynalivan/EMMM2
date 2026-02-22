use tauri::State;
use crate::error::CommandError;
use crate::AppState;

#[tauri::command]
pub async fn my_command_name(
    state: State<'_, AppState>,
    id: String
) -> Result<String, CommandError> {
    // 1. Log entry
    log::info!("Executing my_command_name with id: {}", id);

    // 2. Logic (Awaitable)
    let result = state.db.get_item(&id).await?;

    // 3. Return
    Ok(result)
}
