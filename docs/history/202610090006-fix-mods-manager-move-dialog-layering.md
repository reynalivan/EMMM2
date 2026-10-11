# Fix Mods Manager Move Dialog Layering

## Context

Native Mods Manager smoke testing showed that the bulk **Move to Object** dialog rendered below the preview panel. Its confirmation button was visible but could not receive pointer input.

## Changes

- Open `MoveToObjectDialog` through the shared native-dialog synchronization hook so it enters the browser top layer.
- Add a regression unit test proving the dialog invokes `showModal()`.
- Extend the native workspace smoke coverage for bulk move and bulk safety actions, and make its fixture setup independent of hidden search controls.
- Keep WebView2 COM handles out of the async portion of native download confirmation, restoring the native E2E build.

## Validation

- `cargo check --manifest-path src-tauri/Cargo.toml` with `SQLX_OFFLINE=true`
- `pnpm exec vitest run src/widgets/object-sidebar/modals/MoveToObjectDialog.test.tsx --reporter=verbose`
- `pnpm exec wdio run wdio.conf.ts --spec tests/e2e/specs/client-workspace-smoke.e2e.ts` — 22 passing
