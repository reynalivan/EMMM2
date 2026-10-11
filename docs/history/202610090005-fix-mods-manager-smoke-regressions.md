# Fix Mods Manager Smoke Regressions

## Context

The Mods Manager smoke run exposed a native-browser title race and E2E cases that no longer matched current IPC contracts.

## Changes

- Buffer early browser navigation events until their native tab is present, preserving document titles emitted during tab creation.
- Permit dynamic CSS elements and attributes without relaxing script or network CSP directives.
- Align game auto-detection, hotkey, bulk operations, classification, browser homepage, and workspace-menu E2E calls with current contracts.

## Impacted Files

- `src/pages/browser/components/BrowserPage.tsx` (modified)
- `src/pages/browser/utils/browserNavigation.ts` (added)
- `src/pages/browser/utils/browserNavigation.test.ts` (added)
- `src-tauri/tauri.conf.json` (modified)
- `tests/e2e/specs/phase1b-game-edit.e2e.ts` (modified)
- `tests/e2e/specs/phase1c-settings-depth.e2e.ts` (modified)
- `tests/e2e/specs/phase2b-navigation-edges.e2e.ts` (modified)
- `tests/e2e/specs/phase3-mod-ops.e2e.ts` (modified)
- `tests/e2e/specs/phase3b-failure-paths.e2e.ts` (modified)
- `tests/e2e/specs/phase6-scan-sync.e2e.ts` (modified)
- `tests/e2e/specs/phase9-security-browser.e2e.ts` (modified)
- `tests/e2e/specs/client-workspace-smoke.e2e.ts` (modified)

## Goal

Discover tab titles survive early native events, and smoke tests exercise the supported command payloads.

## Impact

Dynamic UI styles can apply under Tauri CSP; executable scripts and network sources remain restricted. Native E2E execution remains blocked by an unrelated Rust `Send` error in the in-progress download handler.
