---
description: Refactor code safely. Enforces Modularity, SRP, and Code Simplification.
---

1.  **🛡️ PRE-FLIGHT CHECK**
    - **Check:** files > 350 lines?
    - **Check:** Tests exist & pass? (NO? Write tests first).
    - **Action:** `git commit -am "chore: save state before refactor"`.

2.  **🔍 ANALYSIS & STRATEGY**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-refactoring/SKILL.md` (Patterns).
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/code-simplifier/SKILL.md` (Complexity).
    - **Tool:** Use `sequential-thinking` MCP for methodical architectural tracing before large splits.
    - **Tool:** Use `narsil-mcp` (e.g., `find_dead_code`) to safely groom old Rust functions.
    - **Plan:** Decide whether to Extract Method, Component, or Hook following Domain-Driven structure (`src/features/`).

3.  **🔨 EXECUTION**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/atomic-fs/SKILL.md` (Safe Moves).
    - **Rule:** **No Truncation** (Zero-Truncation Policy).
    - **Action:** Rewrite for Clarity and maintain strict separation between frontend (`src/`) and backend (`src-tauri/`).

4.  **✅ VERIFICATION**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.
    - **Action:** Run All Tests & Lint.

5.  **💾 COMMIT**
    - `refactor(<scope>): <details>`
