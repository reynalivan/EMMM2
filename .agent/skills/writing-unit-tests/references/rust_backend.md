# Rust Backend Testing Templates

## 1. Pure Unit Test (Logic)

**Use for:** Parsers, Hashing, Utilities.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_calculates_hash_correctly() {
        let input = "test content";
        let expected = "hash_value";
        assert_eq!(calculate_hash(input), expected);
    }
}
```

## 2. Async Service Test

**Use for:** Logic involving I/O or Tasks.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_scans_directory() {
        // Setup
        let temp_dir = tempfile::tempdir().unwrap();

        // Act
        let result = scan_dir(temp_dir.path()).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

## 3. Database Integration Test (CRITICAL)

**Use for:** All Repository methods or logic touching SQLite.
**Rule:** MUST use `#[sqlx::test]` for isolation and transaction rollback.

```rust
use sqlx::SqlitePool;

#[sqlx::test]
async fn test_mod_repository_insert(pool: SqlitePool) {
    // 1. Arrange (Repo uses the injected test pool)
    let repo = ModRepository::new(pool);
    let new_mod = Mod::new("Genshin Impact", "Fix.ini");

    // 2. Act
    let result = repo.create(new_mod).await;

    // 3. Assert
    assert!(result.is_ok());
    let saved = repo.find_by_name("Fix.ini").await.unwrap();
    assert_eq!(saved.name, "Fix.ini");
}
```
