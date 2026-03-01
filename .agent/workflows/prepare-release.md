---
description: Prepare a release. Enforces Changelog generation and Verification.
---

1.  **🛡️ QUALITY GATE (Zero-Tolerance)**
    - **Action:** Execute `/verify-quality`.
    - **Action:** MUST compile Tauri Release Build (`npm run tauri build`) and pass ALL E2E UI Specs (`npm run test:e2e`).
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/verification-before-completion/SKILL.md`.

2.  **📜 CHANGELOG**
    - **Skill:** `e:/Dev/EMMM2NEW/.agent/skills/changelog-generator/SKILL.md`.
    - **Action:** Generate notes from git history.
    - **Update:** `CHANGELOG.md`.

3.  **📦 PACKAGING**
    - **Action:** `pnpm tauri build`.
    - **Check:** Artifacts generated in `src-tauri/target/release/bundle`.

4.  **🚀 NOTIFY**
    - **Output:** Release Nodes + Artifact Paths.
