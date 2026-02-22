# Service Pattern (Clean Architecture)

## 1. Structure
```
src-tauri/src/
├── commands/       # API Layer (Controller)
│   └── mod_cmds.rs
├── services/       # Business Logic Layer
│   └── mod_scanner.rs
└── database/       # Data Access Layer
    └── mod_repo.rs
```

## 2. Implementation Guide

### A. The Repository (Data Access)
Responsible ONLY for talking to SQLite. Partition by Table.
```rust
// database/mod_repo.rs
pub struct ModRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ModRepository<'a> {
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Mod>, sqlx::Error> {
        sqlx::query_as!(Mod, "SELECT * FROM mods WHERE id = ?", id)
            .fetch_optional(self.pool)
            .await
    }
}
```

### B. The Service (Business Logic)
Orchestrates Repositories and Logic (Hashing, Parsing).
```rust
// services/mod_service.rs
pub struct ModService;

impl ModService {
    pub async fn toggle_mod(pool: &SqlitePool, id: &str) -> AppResult<()> {
        let repo = ModRepository::new(pool);
        let mod_item = repo.get_by_id(id).await?.ok_or(AppError::NotFound)?;
        
        // Logic: Rename Folder -> Update DB
        fs_ops::rename(&mod_item.path, &new_path)?;
        repo.update_status(id, "ENABLED").await?;
        
        Ok(())
    }
}
```

### C. The Command (Presentation)
Accepts Input -> Calls Service -> Returns JSON.
```rust
// commands/mod_cmds.rs
#[tauri::command]
pub async fn toggle_mod(
    state: State<'_, AppState>,
    id: String
) -> Result<(), String> {
    // Validation
    if id.is_empty() { return Err("Invalid ID".into()); }

    // Execution
    ModService::toggle_mod(&state.db, &id).await
        .map_err(|e| e.to_string())
}
```
