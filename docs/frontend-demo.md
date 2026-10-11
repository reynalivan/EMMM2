# Frontend Demo Mode

Frontend demo mode lets the team review the real EMMM frontend in a browser without connecting to game files, native services, or user data. It is a fixture layer, not a second interface.

## Choose the verification surface

The default for EMMM Windows application testing is **native E2E**, using
WebdriverIO, `tauri-driver`, WebView2 and the isolated executable built by
`wdio.conf.ts`. Follow [the native E2E skill](../.agent/skills/e2e-automation/SKILL.md)
and [workflow](../.agent/workflows/e2e-testing.md).

Use native E2E for application UI, startup/reopen, loading states, performance,
IPC, filesystem, watcher, storage, import, persistence and recovery checks.
Do not open a browser or start this demo merely because the app contains React.
If native testing is blocked, report the build/driver/runtime blocker and the
unverified scenario; do not silently fall back to browser QA.

This document's browser instructions apply only when the user explicitly requests
demo, fixture or browser review. Demo results describe in-memory frontend
behavior and cannot certify native app behavior. Unit tests may still protect
fixture compatibility without launching the demo. Documentation-only changes
require documentation validation, not native E2E.

## Start the demo

For an explicitly requested browser demo review, run:

```sh
pnpm dev:demo
```

Use the local URL printed by Vite. This command runs Vite with `--mode demo`.

Demo mode is development-only. `pnpm exec vite build --mode demo` must fail. A normal `pnpm build` must succeed without embedding demo fixtures.

## Contract

Demo mode renders the same `App`, `AppShell`, `TopBar`, routes, pages, modals, wizards, and controls as the application. The only substitution is data and selected side-effect adapters.

Do not add any of the following for demo work:

- `DemoApp`, demo shell, alternate sidebar, top navigation, or route tree
- Standalone mock page, dashboard, modal, wizard, or replacement control
- CSS that imitates an existing screen instead of rendering that screen

Use the existing component and feed it fixture data through its existing query gateway or typed command path. If an existing screen cannot display the scenario, improve that real screen in a normal product task with explicit approval. Do not work around it with demo-only UI.

## Safety boundary

- Start demo mode only through `pnpm dev:demo`.
- Fixture modules live in `src/demo/` and are selected by Vite aliases only in demo mode.
- Production aliases resolve to `src/demo/runtime/*.disabled.ts`.
- `src/shared/lib/appMode.ts` rejects demo mode outside the Vite development server.
- `src/shared/api/tauri/bindings.ts` routes handled demo commands to in-memory fixtures. Commands without a demo handler reject and never reach native Tauri.
- Demo-only page guards prevent direct native listeners, file drops, downloads, browser webview work, disk repair, and dialog hosts from reaching the operating system.
- A demo mutation may update module-local fixture state so the existing UI can show feedback. Reloading resets that state. It must not touch real settings, game directories, databases, credentials, processes, or services.

## Fixture locations

| Need                                                                                  | Location                         |
| ------------------------------------------------------------------------------------- | -------------------------------- |
| Bootstrap the existing app with fixture settings                                      | `src/demo/bootstrap.ts`          |
| Dashboard stats, activity, and active key mappings                                    | `src/demo/dashboard.ts`          |
| Game settings and game switcher data                                                  | `src/demo/game.ts`               |
| Mods Manager objects, folder list, previews, and safety states                        | `src/demo/workspace.ts`          |
| Collections, Mod Inbox, Storage Optimizer, Discover, Downloads, and command responses | `src/demo/commands.ts`           |
| Production-safe fallbacks                                                             | `src/demo/runtime/*.disabled.ts` |

Keep data compatible with `src/shared/api/tauri/bindings.gen.ts`. Prefer a typed fixture constant or factory. Do not invent a second frontend model, use `any`, or hide a mismatch with a type assertion.

## Updating fixture content

1. Find the existing page, gateway, query, and command contract.
2. Add or adjust only the typed fixture needed for the scenario.
3. Add a command handler when the existing UI invokes a command. Keep its effects in memory.
4. Preserve the existing UI state flow. For example, use the real game switch action, collection controls, filters, and mod selection rather than replacing them.
5. Guard any direct native side effect that is outside the typed command layer.
6. Add or update a focused test when fixture logic has branches, state, or a public contract.

Fixture data should cover real review states. Use varied names and lengths, multiple game choices, active and inactive records, safe and unsafe records, empty results where a page supports them, and recoverable error or loading states when the existing view exposes them. Label fixture-only copy honestly when it appears to a reviewer.

## UI design verification

Demo mode is for reviewing the product interface, not for designing an alternate interface.

For explicitly requested browser demo review, check the affected existing screens:

- Global shell, sidebar, top bar, and game switcher
- Dashboard, Mods Manager, Collections, Mod Inbox, and Storage Optimizer
- Discover, Downloads, Settings, Onboarding, and every reachable modal or wizard
- Long content, empty states, loading states, and error states where supported
- Keyboard navigation, visible focus, dialogs closing with Escape, and responsive reflow without horizontal clipping

Use the existing product visual language as the source of truth. Preserve its hierarchy, spacing, controls, and component composition. If a screenshot, history entry, or design document establishes a different intended layout, compare the current screen against that evidence and update the real component only after the task explicitly includes a product UI change.

## Required checks

Run checks appropriate to the fixture change:

```sh
pnpm test -- --run src/demo/workspace.test.ts
pnpm exec tsc --noEmit
pnpm build
git diff --check
pnpm exec vite build --mode demo
```

The last command is a negative test. It passes only when the command exits with the development-only error.

If browser demo review was explicitly requested, inspect the affected screens,
controls, fixture game switches, keyboard behavior and relevant layouts. Record
the fixture-only coverage and console findings in the history entry. For normal
application changes, run focused native Windows E2E instead.
