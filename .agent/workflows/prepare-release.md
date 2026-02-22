---
description: Prepare a release. Enforces Changelog generation and Verification.
---

1.  **🛡️ QUALITY GATE**
    -   **Action:** Execute `/verify-quality`.
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.

2.  **📜 CHANGELOG**
    -   **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/changelog-generator/SKILL.md`.
    -   **Action:** Generate notes from git history.
    -   **Update:** `CHANGELOG.md`.

3.  **📦 PACKAGING**
    -   **Action:** `pnpm tauri build`.
    -   **Check:** Artifacts generated in `src-tauri/target/release/bundle`.

4.  **🚀 NOTIFY**
    -   **Output:** Release Nodes + Artifact Paths.
