---
description: Create End-to-End (E2E) tests for Tauri apps using WebdriverIO and Mocha.
---

- **PLAN**:
  - Basis: Feature spec + ./.agent/skills/e2e-automation/SKILL.md.
  - OS Check: Windows URI validation.
- **ACT**:
  - Scripting: Create `.e2e.ts` in `src-tauri/tests/`.
  - Sync: Suppress file watcher.
- **REFLECT**:
  - Verify: `npm run test:e2e`. Bridge must hold.
  - Commit: `test(e2e): <feature>`
