# Security Checklist

## Backend (Rust/Tauri)

- **SQL Injection**: Are all SQLx queries using parameters (`bind()` or `?`)? **NEVER** usage of `format!()` for SQL.
- **Command Guard**: Are `#[tauri::command]` inputs validated before use?
- **Unsafe Code**: Is any `unsafe {}` block absolutely necessary and documented with `// SAFETY:`?
- **Path Traversal**: Are file paths validated to prevent accessing outside allowed directories?
- **Secret Storage**: Are API keys/secrets loaded from Env Vars, NOT hardcoded?

## Frontend (React)

- **XSS**: usage of `dangerouslySetInnerHTML`? (Establish strict justification).
- **Deps**: `npm audit` clean?
- **State**: Is sensitive data (passwords) stored in `localStorage`? (Forbidden).
