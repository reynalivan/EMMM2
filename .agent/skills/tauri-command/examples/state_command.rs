use tauri::{State, Manager};
use std::sync::Mutex;

// Define State
pub struct AppState {
    pub db_connection: Mutex<Option<String>>, // Simplified for example
}

#[tauri::command]
pub async fn connect_db(state: State<'_, AppState>) -> Result<String, String> {
    // Lock for mutable access
    let mut db = state.db_connection.lock().map_err(|_| "Failed to lock mutex")?;
    
    *db = Some("Connected".to_string());
    
    Ok("Database Connected".to_string())
}

#[tauri::command]
pub async fn read_data(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db_connection.lock().map_err(|_| "Failed to lock mutex")?;
    
    match &*db {
        Some(conn) => Ok(format!("Status: {}", conn)),
        None => Err("Database not connected".to_string()),
    }
}
