# Rust Documentation Standards

## 1. Doc Comments (`///`)
Use for all public structs, enums, and functions.

```rust
/// Calculates the BLAKE3 hash of a file.
///
/// # Arguments
/// * `path` - Absolute path to the file.
///
/// # Returns
/// * `Result<String>` - The hex string of the hash or an error.
///
/// # Example
/// ```rust
/// let hash = hash_file("C:/mod.ini")?;
/// ```
pub fn hash_file(path: &str) -> AppResult<String> { ... }
```

## 2. Module Documentation (`//!`)
Place at the top of `mod.rs` or `lib.rs`.

```rust
//! # Mod Scanner Service
//!
//! Handles the deep inspection of mod folders including:
//! - Recursive file walking
//! - INI parsing
//! - Hash generation
```

## 3. Tauri Commands
Document the Frontend <-> Backend contract.

```rust
/// Scans a directory for mods.
///
/// **Frontend invoke:** `invoke('scan_mods', { path: '...' })`
#[tauri::command]
pub async fn scan_mods(path: String) -> CommandResult<Vec<Mod>> { ... }
```
