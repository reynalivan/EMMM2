---
name: database-design
description: Database schema design, optimization, and migration patterns for SQLite (SQLx). Use for designing schemas, writing migrations, or optimizing queries.
---

# Database Design Skill

Patterns for EMMM2's **SQLite** engine.

## 1. Schema Principles

- **ID**: Use `TEXT PRIMARY KEY` (UUID v4). No `AUTOINCREMENT`.
- **Timestamps**: `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`.
- **Booleans**: `INTEGER` (0=False, 1=True).
- **Strict Mode**: `STRICT` tables where possible (SQLite 3.37+).

## 2. Migration Workflow

Follow `.agent/workflows/db-change.md`.

1.  Generate: `sqlx migrate add <description>`
2.  Edit: Write standard SQL in `migrations/<timestamp>_<desc>.sql`.
3.  Apply: `cargo run` (auto-applies on start).

## 3. Performance (SQLite Specific)

- **WAL Mode**: MUST be enabled (`PRAGMA journal_mode=WAL`).
- **Indexes**: Index FKs and `WHERE` columns.
- **Transactions**: Wrap multiple writes in `BEGIN...COMMIT`.

## References

- [Schema Design Pattern](references/schema_design.md)
- [Optimization Guide](references/optimization.md)
- [Migration Pattern](references/migrations.md)
