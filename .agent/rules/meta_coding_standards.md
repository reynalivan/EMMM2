---
trigger: model_decision
description: Meta Coding Standards Rule - When writing logic, refactoring code, or checking standard compliances.
---

# 🧠 Meta Coding Standards

> **Goal:** Maintain a distinct, modular, and AI-maintainable codebase by restricting complexity and enforcing clarity.

## 0. Naming & Immutability

- **Frontend (TS):** PascalCase (Components/Types), kebab-case (Files/Hooks), camelCase (Vars).
- **Backend (Rust):** snake_case (Files/Vars), PascalCase (Structs/Traits).
- **Immutability:** `const` default. Minimize `mut`. Strong typing with no `any`.

## 1. The 350-Line Limit (Modularity)

**Context:** Large files are hard to read and hard for AI to maintain context.

- **RULE:** No file should exceed **350 lines** of code.
- **ACTION:** If a file approaches this limit, refactor logic into helper functions, hooks (`useLogic`), or separate sub-modules.
- **RULE:** **1 Function = 1 Responsibility.**

## 2. Anti-Overengineering (YAGNI)

**Context:** Complexity kills startups.

- **RULE:** Do not build "Abstract Factories" for simple problems.
- **RULE:** Do not implement features "just in case".
- **RULE:** Use the simplest tool for the job.

## 3. Architecture Alignment (TRD)

**Context:** The codebase must reflect the documented architecture in `@/.docs/trd.md`.

- **Backend (Rust)**:
  - **Services**: Logic must reside in `src-tauri/src/services/`.
  - **Commands**: Only expose logic via `src-tauri/src/commands/`.
  - **Error Handling**: Use `thiserror` and `anyhow` as per TRD.
  - **Concurrency**: Use `tokio` for async I/O and `std::thread` for CPU-bound tasks (Deep Matcher).
- **Frontend (React)**:
  - **Feature Slices**: Organize by feature in `src/features/` (Onboarding, Dashboard, etc.).
  - **State**: Use `Zustand` for global state, `TanStack Query` for server state.

## 4. Control Flow (Fail Fast)

**Context:** Deep nesting hurts readability.

- **RULE:** Use **Guard Clauses** to return early.
- **FORBIDDEN:** Nested `if` statements deeper than 2 levels.
- **PATTERN:**
  ```typescript
  if (!user || !user.active) return;
  save();
  ```

## 5. SRP (One Thing Rule)

**Context:** "God Functions" are impossible to test.

- **RULE:** A function should do **one thing** only.
- **TEST:** Can you describe the function without using the word "and"?

## 6. Composition (The Lego Rule)

**Context:** Inheritance creates brittle chains.

- **RULE:** Prefer **Composition** over Inheritance.
- **PATTERN:** Use `hooks` (functional composition) instead of Class Inheritance.

## 7. No Circular Dependencies (Cycle-Free)

**Context:** A depends on B, and B depends on A.

- **RULE:** All dependencies must flow in one direction (Acyclic).
- **FORBIDDEN:** Circular imports between files or modules.
