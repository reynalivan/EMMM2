---
trigger: model_decision
description: Security Secrets Rule - When dealing with authentication, environment variables, tokens, passcodes, or credentials.
---

# 🔐 Security Rule: Zero Tolerance for Exposed Secrets

> **CRITICAL: All sensitive keys, secrets, and credentials MUST be stored in environment variables. NEVER hardcode them in source code.**

## 🚫 Strictly Prohibited

- **Hardcoded API Keys**: Never paste keys directly into `.ts`, `.rs`, or `.json` files.
- **Embedded Credentials**: No database passwords in connection strings within code.
- **Commiting .env**: Ensure `.gitignore` blocks all `.env` files (except `.env.example`).

## 🛠️ Implementation Guidelines

### 1. Frontend (React/Vite)

- Access publicly safe variables via `import.meta.env.VITE_*`.
- **NEVER** expose backend secrets (like database passwords) to the frontend bundle.

### 2. Backend (Rust/Tauri)

- Use the `dotenv` crate to load variables at runtime or compile time.
- For `tauri.conf.json`, use standard configuration; do not embed secrets.
- Use `std::env::var` to access secrets in Rust.

### 3. File Handling

- **Start**: Clone `.env.example` to `.env`.
- **Git**: Verify `.gitignore` contains:
  ```gitignore
  .env
  .env.local
  *.pem
  *.key
  ```

## 🕵️ Detection Checklist

Before any commit or file creation, verify:

- [ ] No secrets in `const` or `let` assignments.
- [ ] No secrets in comments.
- [ ] No secrets in `console.log`.

**If you find a hardcoded secret: STOP. Rotate the key immediately and refactor.**
