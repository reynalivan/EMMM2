---
trigger: model_decision
description: Project Structure Rule - When creating new files, refactoring folders, or deciding where to put code.
---

# 🏗️ Project Structure Rule

> **Goal:** Strict separation of concerns. Tauri v2 Compliance.

## 1. Top-Level Philosophy

- **Domain-Driven:** Group by Feature (`features/onboarding`), not Type.
- **Separate Worlds:** Frontend (`src`) and Backend (`src-tauri`) share NO code.
- **Source of Truth:** `@/.docs/trd.md`.

## 2. Backend (Rust: `src-tauri/`)

| Path            | Purpose      | Rules                                      |
| :-------------- | :----------- | :----------------------------------------- |
| `src/commands/` | **Bridge**   | Validation only. Calls Services. No Logic. |
| `src/services/` | **Brain**    | Pure Rust Logic. Testable.                 |
| `src/database/` | **Data**     | `sqlx` queries only.                       |
| `capabilities/` | **Security** | **Tauri v2 Permissions**. JSON/TOML files. |

### ⚠️ Strict Constraints

- **main.rs:** Setup ONLY. No logic.
- **FORBIDDEN:** Defining permissions in `tauri.conf.json` (Use `capabilities/`).

## 3. Frontend (React: `src/`)

| Path             | Purpose    | Rules                         |
| :--------------- | :--------- | :---------------------------- |
| `features/`      | **Slices** | Feature-specific UI/Logic.    |
| `components/ui/` | **Atoms**  | Reusable DaisyUI components.  |
| `stores/`        | **State**  | Global Zustand (Theme, User). |

### 📂 Feature Slice (`src/features/xyz/`)

- `components/` (UI), `hooks/` (Logic), `index.ts` (Public API).

## 4. Anti-Patterns

- ❌ **God Components:** Mixing Logic + UI.
- ❌ **Leaking:** DB concepts in UI.
- ❌ **Circular Deps:** Feature A -> Feature B.
