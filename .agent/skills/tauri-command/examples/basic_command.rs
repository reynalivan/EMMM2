use tauri::Window;

#[tauri::command]
pub async fn greet(name: String) -> Result<String, String> {
    // Simulate thinking time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    
    Ok(format!("Hello, {}! Welcome to EMMM2.", name))
}
