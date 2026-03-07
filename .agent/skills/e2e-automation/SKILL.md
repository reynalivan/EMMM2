---
name: e2e-automation
description: Creating WebdriverIO End-to-End Tests for Tauri 2.0 applications.
triggers: ['e2e', 'webdriverio', 'tauri automation testing', 'create e2e test']
---

# 🤖 E2E Automation Skill (WebdriverIO + Tauri)

This skill provides guidelines and patterns for writing GUI integration tests in EMMM2 using WebdriverIO and Tauri's native WebDriver.

## 1. Directory Structure

All E2E spec files MUST be placed in:
`test/specs/*.e2e.ts`

## 2. Global Test Structure

Use standard Mocha `describe` and `it` blocks.

```typescript
import { expect, browser, $ } from '@wdio/globals';

describe('EMMM2 Feature: Mod Toggling', () => {
  it('should enable a disabled mod folder', async () => {
    // 1. Arrange: Find the target element
    const toggleGridItem = await $('[data-testid="mod-item-DISABLED-TestMod"]');

    // 2. Act: Interact
    await toggleGridItem.click();

    // 3. Wait: Backend file operations take time
    await browser.pause(500); // or explicit wait

    // 4. Assert: Check UI state changes
    const enabledItem = await $('[data-testid="mod-item-TestMod"]');
    await expect(enabledItem).toBeExisting();
  });
});
```

## 3. Dealing With Asynchrony (CRITICAL)

Tauri commands (`invoke`) and React State (`zustand`) take time.

- NEVER assume the UI updates instantly.
- **Good:** `await myBtn.waitForClickable({ timeout: 3000 })`
- **Good:** `await browser.pause(500)` when a Rust backend task is manipulating the actual Windows file system.

## 4. Element Selection Rules

DaisyUI elements can sometimes be tricky to select via standard paths.

- **Rule 1:** Prefer identifying elements using `data-testid="..."` attributes injected into React components.
- **Rule 2:** If `data-testid` is missing, use component texts: `await $('button=Apply Collection')`.
- **Rule 3:** Ensure nested elements aren't accidentally masked by absolute-positioned overlays (e.g. Ghost Overlays in DnD).

## 5. Mocking vs Native Execution

WebdriverIO runs against the REAL `emmm2.exe`. If you edit a file during testing, it changes the real disk. Do NOT mock Rust logic during E2E. The intent of E2E in this project is to verify the IPC bridge end-to-end.

## 6. Execution & Build Requirements

To run the automated tests properly, the Agent MUST execute:

1. `npm run tauri build` (Wait until the Executable is created in `/src-tauri/target/release/`)
2. `npm run test:e2e`
