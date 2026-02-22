---
name: tauri-command
description: Standard pattern for Rust Tauri Commands. Use when creating new backend API endpoints or exposing Rust logic to the frontend.
---

# Tauri Command Skill

Standard boilerplate for secure, async, and error-safe Tauri Commands.

## When to use
-   Creating new files in `src-tauri/src/commands/*.rs`.
-   Exposing Rust logic to the React Frontend.
-   Handling heavy computations (Deep Matcher) or File I/O (Scanning).

## Core Rules

### 1. Structure & Location
-   **File**: Place in `src-tauri/src/commands/<domain>.rs`.
-   **Mod**: Register in `src-tauri/src/lib.rs`.
-   **Public**: Functions must be `#[tauri::command]`.

### 2. Async by Default
-   **Rule**: All I/O and Heavy Ops MUST be `async fn`.
-   **Pattern**: Use `tokio::spawn` for background tasks, or `std::thread` for CPU-bound work (like hashing) to avoid blocking the runtime.
-   **Reference**: [async_tasks.md](references/async_tasks.md)

### 3. Error Handling (The "Result" Rule)
-   **Rule**: ALWAYS return `Result<T, String>` or `Result<T, AppError>`.
-   **Forbidden**: `unwrap()` or `expect()` in command logic.
-   **Reference**: [error_handling.md](references/error_handling.md)

### 4. State Management
-   **Rule**: Use `tauri::State<'_, AppState>` to access DB pools or Config.
-   **Pattern**: Inner mutability (`Mutex` / `RwLock`) is required for mutable state.
-   **Example**: [state_command.rs](examples/state_command.rs)
