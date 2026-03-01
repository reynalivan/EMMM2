---
trigger: model_decision
description: Project Structure Rule - When creating new files, refactoring folders, or deciding where to put code.
---

# 🏗️ Project Structure Rule

> **Goal:** Strict separation of concerns based on the TRD (`.docs/trd.md`). Tauri v2 Compliance.

## 1. Top-Level Philosophy

- **Domain-Driven:** Group code by Feature (`src/features/folder_grid`), NOT by Type. Matches specific `req-*.md` documents.
- **Hybrid Architecture:** The UI is optimistic, the File System is the Source of Truth, the DB is an Index Cache.
- **Separate Worlds:** Frontend (`src`) and Backend (`src-tauri`) share NO code logic.

## 2. Backend (Rust: `src-tauri/`)

| Path            | Purpose      | Rules                                                                                      |
| :-------------- | :----------- | :----------------------------------------------------------------------------------------- |
| `src/commands/` | **Bridge**   | Tauri endpoints. Validation/Lock generation only. Calls Services. No complex Domain logic. |
| `src/services/` | **Brain**    | Pure Rust Logic (`scanner`, `file_ops`, `parser`). Testable.                               |
| `src/database/` | **Data**     | `sqlx` models and queries ONLY.                                                            |
| `capabilities/` | **Security** | **Tauri v2 Permissions**. JSON/TOML files defining IPC scopes.                             |

### ⚠️ Strict Constraints

- **main.rs:** Setup/Plugin initialization ONLY. No business logic.
- **FORBIDDEN:** Defining capabilities in `tauri.conf.json` directly (Use `capabilities/`).

## 3. Frontend (React: `src/`)

| Path             | Purpose    | Rules                                                                  |
| :--------------- | :--------- | :--------------------------------------------------------------------- |
| `features/`      | **Slices** | Feature-specific UI/Logic (e.g. `bootstrap`, `objectlist`, `preview`). |
| `components/ui/` | **Atoms**  | Reusable DaisyUI + Tailwind 4 atomic components.                       |
| `hooks/`         | **Shared** | Custom utility or TanStack Query custom hooks.                         |
| `stores/`        | **State**  | Global Zustand states (`appStore`, `sessionStore`).                    |

### 📂 Feature Slice (`src/features/xyz/`)

Should be self-contained: `components/` (UI), `hooks/` (Local Logic), `index.ts` (Public API).

## 4. Anti-Patterns

- ❌ **God Components:** Mixing heavy state/query logic directly into JSX renders.
- ❌ **State Desync:** Trusting the UI/DB over the actual File System.
- ❌ **Circular Dependencies:** Feature A importing directly from Feature B internals.
