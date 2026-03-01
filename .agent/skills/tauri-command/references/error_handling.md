# Error Handling Patterns

## 1. The String Pattern (Simple)

For most commands, returning `Result<T, String>` is sufficient. The frontend receives the error string directly.

```rust
#[tauri::command]
pub fn simple_command() -> Result<String, String> {
    if condition {
        Ok("Success".into())
    } else {
        Err("Something went wrong".into())
    }
}
```

## 2. The `anyhow` Pattern (Recommended)

Use `anyhow` for easy error propagation, then map it to `String` at the boundary.

```rust
#[tauri::command]
pub async fn complex_command() -> Result<String, String> {
    internal_function().await.map_err(|e| e.to_string())
}

async fn internal_function() -> anyhow::Result<String> {
    // ... use ? operator freely
    Ok("Data".into())
}
```

## 3. The Custom Enum Pattern (Advanced)

If the frontend needs to react differently to specific errors (e.g., "AuthFailed" vs "NetworkError").

```rust
use serde::Serialize;

#[derive(Serialize)]
pub enum CommandError {
    DatabaseError(String),
    IOError(String),
    NotFound,
}

#[tauri::command]
pub fn structured_error() -> Result<(), CommandError> {
    Err(CommandError::NotFound)
}
```
