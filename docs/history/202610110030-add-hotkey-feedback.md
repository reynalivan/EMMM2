# Add global hotkey feedback

## Context

Show collection origin/target and Safe Mode changes on every EMMM page, with
actual control hints and independently configured game notification surfaces.

## Changes

- Publish typed, original-root-owned receipts only after committed actions.
- Add global nonmodal feedback, actual reload-state helpers and default-off native
  notifications. Keep 3DMigoto status independently toggleable with five bounded
  helper lines. Native presentation never transfers focus or rolls back mods.
- Make readiness/expiry/hide/destruction token-safe and recover transient snapshot
  failures. Keep browser content below notices without stealing keyboard focus.
- Bundle a lightweight notice entry; guard actual HTML emission/main bootstrap.
- Reject negative client-monitor extents; correct global-hotkey text in EN/ID/ZH.

## Impacted files

Frontend added:

- `src/features/hotkey-feedback/{HotkeyNoticeHost.tsx,store.ts,index.ts,HotkeyNoticeHost.test.tsx}`
- `src/shared/lib/{hotkeyNotice.ts,hotkeyNotice.test.ts}`
- `src/app/hotkey-notice/{main.ts,renderer.ts,renderer.test.ts,notice.css}`
- `src/tests/testing/hotkeyNoticeFixture.ts`; `hotkey-notice.html`
- `scripts/bootstrap-entry.test.mjs`

Frontend modified:

- `src/app/entrypoint/App.tsx`; `src/shared/api/tauri/{bindings.gen.ts,events.ts}`
- `src/pages/settings/components/tabs/{HotkeyTab.tsx,HotkeyTab.test.ts}`
- `src/pages/browser/{browserSurfacePresentation.ts,browserSurfacePresentation.test.ts}`
- `src/pages/browser/components/BrowserPage.tsx`
- `src/pages/browser/hooks/{useWebviewSync.ts,useWebviewSync.test.tsx}`
- `src/shared/i18n/locales/{en,id,zh}/settings.json`
- `vite.config.ts`; `scripts/check-bootstrap-bundle.mjs`

Backend added/modified:

- `automation/application/hotkeys/{notice.rs,mod.rs,manager.rs,cycle_preset.rs,safe_mode.rs,tests/hotkey_tests.rs}`
- `automation/adapters/tauri/{hotkey_notice.rs,mod.rs}`; `automation/api.rs`
- `automation/application/keyviewer/generator/{status.rs,ini.rs}`
- `automation/application/keyviewer/tests/generator/status_tests.rs`
- `reconciliation/{adapters/tauri/runtime_sync.rs,application/disk_reconcile/types.rs}`
- `system/{api.rs,application/game_detector.rs,application/app/post_apply.rs}`
- `src-tauri/{src/lib.rs,Cargo.toml,AGENT.md}`
- `src-tauri/permissions/{app-commands.toml,hotkey-notice.toml}`
- `src-tauri/capabilities/hotkey-notice.json`

Tests/docs:

- `tests/e2e/{specs/global-hotkeys-native.e2e.ts,support/hotkey-input-window.cs}`
- `docs/plans/hotkey-feedback/{implementation_plan.md,validation.md}`; this history
- Ignored research/config/evidence/snapshot files in `.audit-output/`

## Goal and impact

60 focused frontend tests, 70 focused Rust tests and registry/export checks passed.
Native S passed Home/Settings notices, opt-in/out, expiry, OS hotkey disk/DB effects
and unchanged foreground, with 1572 inputs verified. Owned processes/fixture cleaned.
No dependency, migration, real-library mutation, install, commit or push.
Hardware click-through, multi-monitor visuals, real loader rendering and exclusive
fullscreen visibility are not certified. See validation for post-S boundary/copy
changes and the fact the installed app remains on its earlier executable.
