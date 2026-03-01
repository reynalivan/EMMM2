---
trigger: model_decision
description: Technology Stack Rule - When checking tech stack, adding dependencies, checking versions, or upgrading libraries.
---

# 🥞 Technology Stack Standards

> **Goal:** Strictly enforce architecture and dependency rules matching `.docs/trd.md`.

## Core Setup

- **Tauri v2** + **Rust** Backend + **Vite/React 19/TS 5+** Frontend.
- **Strict Rule:** ALWAYS use `pnpm`. NEVER `npm`, `yarn`, or `bun`.

## Frontend Constraints

- UI: **Tailwind v4** + **daisyUI 5** + **lucide-react**. Use semantic `btn` classes.
- Architecture: **Frontend Alignment**. Keep heavy compute out of JS (offload DP algorithms/hashing to Rust).
- State: `zustand` (global UI), `@tanstack/react-query` (server state).
- Mandatory optimizations: `@tanstack/react-virtual` for > 50 items.
- E2E Testing: **WebdriverIO** (`@wdio/*` via Tauri WebDriver) using Mocha framework. Scripts reside in `test/specs/`.

## Backend Constraints

- Architecture: **Decoupled Business Logic**. Use Data Access Layer (Repositories in `database/`) for `sqlx` queries; keep Business Logic (Services in `services/`) strictly separate.
- Async/Concurrency: `tokio`, `std::thread` / `rayon` for heavy CPU loops.
- Database: `sqlx` (SQLite Tokio). Use `sqlx::query!` macros. Validate Schema & Query Optimization.
- Core crates: `notify` (FS Watcher), `blake3` (Hashing), `compress-tools` (Archive), `image` (Thumbnails), `trash` (Safe Delete).

## Dependency Rules

- Follow `pnpm-lock.yaml` and `Cargo.lock`.
- **FORBIDDEN:** `lodash`, `moment`, `axios`. Use native JS or `date-fns`/`fetch` instead.
