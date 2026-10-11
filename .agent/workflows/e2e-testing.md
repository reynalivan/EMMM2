---
description: Verify the native EMMM Windows application with WebdriverIO, tauri-driver and WebView2.
---

1. Read `.agent/skills/e2e-automation/SKILL.md`, the relevant specs and their
   helpers. Choose the smallest native scenario that verifies the changed flow.
2. Add or update specs in `tests/e2e/specs/*.e2e.ts`. Reuse the existing app and
   owned game fixtures; keep watchers, readiness, IPC and recovery gates active.
3. Run the spec through the existing harness:

   ```powershell
   rtk proxy corepack pnpm exec wdio run wdio.conf.ts --spec tests/e2e/specs/startup-reopen.e2e.ts
   ```

   Replace the example with the relevant spec. `wdio.conf.ts` rebuilds the
   embedded frontend/native binary using `src-tauri/tauri.e2e.conf.json`, verifies
   isolation and starts the owned driver. Do not skip its build or lifecycle hooks.

4. Assert visible UI and relevant real side effects after readiness. For restart
   tests use actual native process/session restarts; navigate the new WebView and
   wait for IPC before reading state. A route reload is not a process restart.
5. Let the harness stop its owned native process tree before fixture cleanup.
   Report the exact spec, result, build flavor and any unverified scenarios.

Browser demo/Playwright QA requires an explicit user request and is not a fallback
for unavailable native tooling. A failing build, driver or app remains a reported
blocker; never reuse the installed production app or reset its database. This
workflow does not authorize installing software, committing, pushing or deploying.
