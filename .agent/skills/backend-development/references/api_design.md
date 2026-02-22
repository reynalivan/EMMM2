# API Design (Tauri Commands)

## 1. Naming Conventions
-   **Commands**: `verb_noun` (e.g., `get_users`, `create_project`).
-   **Arguments**: `snake_case`. Matches frontend `invoke` args.

## 2. Request/Response DTOs
Avoid passing 10 arguments. Use a Struct.

```rust
// Rust
#[derive(Deserialize)]
pub struct CreateUserDto {
    pub username: String,
    pub email: String,
    pub role: String,
}

#[tauri::command]
pub async fn create_user(payload: CreateUserDto) -> ...
```

```typescript
// Frontend
invoke('create_user', { payload: { username: '...', ... } })
```

## 3. Standard Response Format
For complex data, wrap results.

```json
{
  "data": { ... },
  "meta": {
    "count": 100,
    "page": 1
  }
}
```
Or simply return the data directly if it's a list.

## 4. Error Format
Map Rust errors to a standard string or object.
-   **Simple**: `Err("User not found".to_string())`
-   **Typed**: `Err(AppError::Validation { field: "email" })` -> Serializes to JSON for frontend parsing.
