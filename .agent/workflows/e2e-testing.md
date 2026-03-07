---
description: Create End-to-End (E2E) tests for Tauri apps using WebdriverIO and Mocha.
---

# 🤖 E2E Testing Generation Workflow

This workflow guides the AI Agent in creating high-quality E2E tests for EMMM2.

**Objective:** Create high-level user journey E2E tests based on reference test cases, using WebdriverIO with Tauri WebDriver native integration.
**Scope:** High-level user journeys crossing the IPC bridge (e.g., clicking 'Enable' in the React UI and verifying the actual folder rename on the host OS).

## 1. 🔍 REQUIREMENTS & SCOPE ANALYSIS

- **Action:** Open and read the relevant `tc-*.md` file from `e:/Dev/EMMM2NEW/.docs/.testcase/`.
- **Action:** Identify the high-level user journeys that involve both the React Frontend (UI interaction) and the Rust Backend (OS-level changes, IPC).
- **Rule:** Do NOT test granular implementation details (unit tests). Focus on overarching behaviors simulating a real user (e.g., clicking a button, waiting for an IPC call, verifying a file renamed on disk).
- **Dependency:** Read `e:/Dev/EMMM2NEW/.agent/skills/e2e-automation/SKILL.md` for WebdriverIO syntax and best practices.

## 2. 🧪 SPEC DRAFTING

- **Action:** Create a new test spec file in `test/specs/[feature_name].e2e.ts`.
- **Framework:** **WebdriverIO** + **Mocha** (`describe`, `it`).
- **Tauri Integration:** Use the native Tauri WebDriver context. Avoid mocking Rust logic. The test must execute against the real `emmm2.exe` to verify the IPC bridge end-to-end.
- **Node.js Integration:** Use native Node.js modules like `fs` or `fs/promises` in the test script to verify actual host OS consequences (e.g., verifying a physical folder was renamed from `MyMod` to `DISABLED MyMod`).

## 3. 🖱️ UI INTERACTION & SYNCHRONIZATION

- **Selectors:** Prefer `data-testid="..."` for interacting with the React UI.
- **Wait Strategy:** The IPC bridge and Rust backend File I/O take time. Never assume instant updates.
  - Use `await myElement.waitForClickable({ timeout: 5000 })`.
  - Use `await browser.pause(...)` when waiting for backend Rust processes (e.g., scanning or extracting ZIP files) to complete before verifying the file system.
- **Tone Rule:** NO ADVERB HALLUCINATIONS in descriptions or console logs. Keep test cases strictly technical (e.g., "should rename folder to DISABLED prefix on toggle", not "should seamlessly successfully gracefully rename").

## 4. 🏗️ EXECUTION & VALIDATION

- **Build Step:** Ensure the Tauri binary is compiled (`pnpm run tauri build` or `npm run tauri build`) before running new tests.
- **Run Command:** Use `pnpm run test:e2e` (or `npm run test:e2e`) to execute the spec.
- **Troubleshooting:** If the test fails, verify if the element existed but was hidden by an animation mask (Framer Motion) or if the IPC wait time was too short and fix the code until solved

## 5. 💾 COMMIT

- **Action:** Commit the newly created E2E spec.
- **Message:** `test(e2e): add specs for <feature> based on <tc-name>`
