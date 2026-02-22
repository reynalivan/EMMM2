---
trigger: model_decision
description: Database Standards Rule - When designing schemas, writing SQL queries, or running DB migrations.
---

# 🗄️ Database Standards (SQLite + SQLx)

> **Goal:** Ensure data integrity, query performance, and type safety across the application.

## 1. Schema Design

- **Naming**: `snake_case` for all tables and columns.
- **Primary Keys**: MANDATORY. Prefer `TEXT` (UUID) or `INTEGER` (Auto Inc) depending on synchronization needs.
- **Foreign Keys**: ALWAYS define FK constraints to enforce integrity.
- **Soft Delete**: Use `deleted_at` (`DATETIME` nullable) instead of physical DELETE for critical user data.

## 2. Migrations (`sqlx`)

- **Architecture**: We use **`sqlx::migrate!()`** built into `lib.rs`. This executes all SQL files in `src-tauri/migrations/` at compile-time securely.
- **Forbidden**: NEVER use `tauri_plugin_sql` for migrations.
- **Idempotency & Strictness**: `sqlx::migrate!` is strict. NEVER write migration scripts that swallow errors. If a script fails (e.g., `ALTER TABLE` duplicate column), it is a fatal error. Existing user DBs must be reset.
- **Naming**: `{timestamp}_{description}.sql` (e.g., `202401010000_create_mods.sql`).
- **Immutability**: NEVER edit an applied migration file. Create a new one.

## 3. Query Performance

- **Indexing**:
  - Must Index: Foreign Keys, Columns used in `WHERE`, `JOIN`, `ORDER BY`.
  - Avoid Over-indexing: Don't index low-cardinality flags (Booleans) unless part of a composite.
- **Forbidden**: `SELECT *` in production code. Always specify columns (`SELECT id, name`).
- **N+1 Problem**: Use `JOIN` or batch queries (`WHERE id IN (...)`) instead of looping queries.

## 4. Type Safety & SQLx

- **Macros**: Use `sqlx::query!` (checked at compile time) over `sqlx::query`.
- **Binding**: ALWAYS use parameter binding (`?` or `$1`). NEVER string concatenation (SQL Injection risk).
- **Struct Mapping**: Map rows directly to Rust Structs using `query_as!`.

## 5. Transactions

- **Atomicity**: Any operation affecting > 1 table or row MUST use a Transaction.
- **Scope**: Keep transactions short. Do not perform File I/O inside a DB transaction if possible to avoid locking.

## 6. Local Development

- **Seed Data**: Maintain a separate `seed.sql` for populating dev DB with realistic test data.
- **Reset**: Ability to `sqlx database reset` safely in dev environment.

## 7. Architecture (Frontend vs Backend)

- **Backend-Exclusive**: ALL database access (CRUD) MUST be implemented in the Rust Backend using `sqlx::query!` and `QueryBuilder`.
- **Frontend Ban**: NEVER use `tauri-plugin-sql` or attempt to execute Raw SQL queries from TypeScript.
- **Communication**: The frontend MUST use Tauri `invoke('command_name', { ... })` and rely on the Rust backend for any database retrieval or manipulation.
