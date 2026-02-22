---
description: Start a feature safetly. Enforces Impact Analysis and Documentation.
---

1.  **📋 PLANNING**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/ask-questions-if-underspecified/SKILL.md`.
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/writing-plans/SKILL.md`.
    -   **Action:** Create `.docs/plans/[Date]-[Name].md`.

2.  **🔍 IMPACT ANALYSIS**
    -   **Tech Check:**
        -   UI Grid? `e:/Dev/EMMM2NEW/.agent/skills/virt-grid/SKILL.md`.
        -   INI Parse? `e:/Dev/EMMM2NEW/.agent/skills/ini-parser/SKILL.md`.
    -   **Data:** Check `src-tauri/migrations/` (Schema Conflict?).

3.  **🧪 KICKOFF**
    -   **Action:** Execute `/start-epic` (if large) or start `/tdd-cycle` (if small).

4.  **📚 DOCS SYNC**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-documentation/SKILL.md`.
    -   **Action:** Update `.docs/trd.md` (Arch).
    -   **Action:** Add to `task.md`.

5.  **✅ VERIFICATION**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
