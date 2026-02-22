# Rust TDD Patterns

## 1. Unit Tests (Pure Logic)
Location: Bottom of the file in `mod tests`.
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_normalizes_mod_names() {
        // Red
        let input = "[Genshin] Raiden Shogun";
        // Green
        let output = normalize(input);
        assert_eq!(output, "raiden shogun");
    }
}
```

## 2. Database Tests (Integration)
Use `#[sqlx::test]` for automatic transaction rollback. **NEVER** mock the database; use the real SQLite in-memory or file.
```rust
#[sqlx::test]
async fn test_create_game(pool: SqlitePool) {
    // Red: Query doesn't exist yet
    let repo = GameRepository::new(pool);
    let game = repo.create("Genshin Impact").await.unwrap();
    
    // Green: Assert result
    assert_eq!(game.name, "Genshin Impact");
}
```

## 3. Mocking Dependencies
Use traits to allow mocking external systems (File System, HTTP).
```rust
trait FileSystem {
    fn read(&self, path: &str) -> Result<String>;
}

struct RealFs;
#[cfg(test)]
struct MockFs; // Use for testing
```
