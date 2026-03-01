---
trigger: model_decision
description: Troubleshooting Rule When debugging crashes, handling runtime errors, researching bugs, or fixing build and lint errors.
---

# 🚨 Troubleshooting & Repair Standards

> **Goal:** Unified error handling, safe codebase repairs (Root Cause Fixes), and strictly verified assumptions.

## 1. Research & Assumptions (No Guessing)

- **Protocol:** Analyze -> Research (Context7/Web) -> Check `tech_stack.md`.
- **Anti-Assumption:** Never guess imports or API props. If uncertain, look it up via tools before coding.

## 2. Error Handling

- **Backend (Rust):** Return `Result<T, CommandError>`. Use `?` operator. No `panic!`. Use `thiserror`. Log `warn!` or `error!` appropriately.
- **Frontend (TS):** `try-catch` all Tauri `invoke` calls. `catch (e) {}` is FORBIDDEN. Always log and show UI feedback (Toast).
- **E2E (WebdriverIO):** If `npm run test:e2e` fails to launch the app, ensure you have built the application (`npm run tauri build`) first. Check browser logs (`browser.getLogs('browser')`) for unhandled React mount failures or stale elements.

## 3. Repairing Code (Anti-Patch)

- **No Lazy Fixes:** `@ts-ignore`, `any`, or blindly renaming to `_var` are FORBIDDEN.
- **Integrity:** Fix the Source of Truth (Database Schema or Rust Structs), do not manipulate usage to mask type errors.
- **Verify:** Must Build -> Lint -> Test locally after all fixes. Keep changes scoped precisely to the fix (no unrelated refactors).
