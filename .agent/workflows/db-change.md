---
description: Safe Database Migrations. Enforces SQLx safety and Model Sync.
---

1.  **📝 SCHEMA DESIGN**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/database-design/SKILL.md`.
    - **Action:** `sqlx migrate add <name>`.
    - **Rule:** Idempotent SQL.

2.  **🦀 CODE SYNC**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/backend-development/SKILL.md` (Models).
    - **Action:** Update Rust `struct` models.

3.  **🛡️ SAFETY CHECK**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    - **Action:** `cargo sqlx prepare --check`.
    - **Action:** `cargo check`.

4.  **💾 COMMIT**
    - `feat(db): <migration_name>`
