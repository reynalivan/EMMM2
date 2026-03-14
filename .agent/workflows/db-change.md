---
description: Safe Database Migrations. Enforces SQLx safety and Model Sync.
---

- **PLAN**:
  - Research: ./.agent/skills/database-design/SKILL.md.
  - Rule: Idempotent SQL only.
- **ACT**:
  - Migration: `sqlx migrate add <name>`.
  - Sync: Update models (./.agent/skills/backend-development/SKILL.md).
- **REFLECT**:
  - Verify: `cargo sqlx prepare --check`.
  - Commit: `feat(db): <migration_name>`
