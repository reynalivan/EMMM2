---
description: Start a new Requirement/Feature safely. Enforces Impact Analysis and Documentation.
---

1.  **📋 PLANNING**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/ask-questions-if-underspecified/SKILL.md`.
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-plans/SKILL.md`.
    - **Tool:** Query `memory` or `supermemory` to recall past technical decisions or user preferences regarding similar features.
    - **Action:** Read the relevant `.docs/requirements/req-*.md` specification to fully understand the scope.

2.  **🔍 IMPACT ANALYSIS**
    - **Tech Check:**
      - UI Grid? `e:/Dev/EMMM2NEW/.agent/skills/virt-grid/SKILL.md`.
      - INI Parse? `e:/Dev/EMMM2NEW/.agent/skills/ini-parser/SKILL.md`.
      - Backend/Rust? Use `mcp_narsil-mcp_find_symbols` or `mcp_narsil-mcp_get_project_structure` for deep analysis.
      - External API? Query `context7` / `jina-mcp-server`.
      - UI Needs? Query `daisyui` / `shadcn-ui-server`.
    - **Data:** Check `src-tauri/migrations/` (Schema Conflict?).

3.  **🧪 KICKOFF**
    - **Action:** Execute `/start-epic` (for major features) or `/tdd-cycle` (for smaller units).

4.  **📚 DOCS SYNC**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-documentation/SKILL.md`.
    - **Action:** Ensure alignment with `.docs/trd.md` (Arch).
    - **Action:** Add checklist items to `task.md`.

5.  **✅ VERIFICATION**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
