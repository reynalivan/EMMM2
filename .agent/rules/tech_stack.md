---
trigger: model_decision
description: Technology Stack Rule - When checking tech stack, adding dependencies, checking versions, or upgrading libraries.
---

# 🥞 Technology Stack Standards

> **Goal:** Strictly enforce architecture and dependency rules.

## Core Setup

- **Tauri v2** + **Rust** Backend + **Vite/React 19/TS 5+** Frontend.
- **Strict Rule:** ALWAYS use `pnpm`. NEVER `npm`, `yarn`, or `bun`.

## Frontend Constraints

- UI: **Tailwind v4** + **daisyUI 5** + **lucide-react**. Use semantic `btn` classes.
- State: `zustand` (global UI), `@tanstack/react-query` (server state).
- Mandatory optimizations: `@tanstack/react-virtual` for > 50 items.

## Backend Constraints

- Async/Concurrency: `tokio`, `std::thread` / `rayon` for heavy CPU loops.
- Database: `sqlx` (SQLite Tokio). Use `sqlx::query!` macros.
- Core crates: `notify`, `blake3`, `compress-tools`, `image`, `trash`.

## Dependency Rules

- Follow `pnpm-lock.yaml` and `Cargo.lock`.
- **FORBIDDEN:** `lodash`, `moment`, `axios`. Use native JS or `date-fns`/`fetch`.
