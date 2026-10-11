# Global hotkey feedback

Approved by the user on 2026-10-11 (Asia/Jakarta), including control hints added
after the initial approval. Implement after the startup chat releases its source,
compiler and native validation slot. Preserve its automatic Vite chunk splitting.

## Accepted behavior

- A prominent nonmodal notice in the global EMMM shell, including Home and every
  route. Show the previous collection as secondary context and the newly applied
  collection as the large highlighted value. Safe Mode shows ON/OFF, not an
  unsupported claim that every unsafe mod is enabled. No focus transfer or forced
  restore of minimized EMMM. Success follows committed operations, not key presses.
- Keep the existing independently saved `preset_status_overlay_enabled` Settings
  toggle for 3DMigoto status. Publication follows the existing latest-wins runtime
  queue and immutable resources. Disabled status and KeyViewer controls remain
  independent. No new loader hook or DLL injection.
- Add a separate default-off `game_notification_overlay_enabled` saved setting.
  Hotkeys remain global when this setting is off. A transient transparent,
  always-on-top notice appears only over the configured foreground game, on that
  game's monitor. It ignores mouse input, cannot activate, and is hidden on focus
  loss, minimize, selection/root changes, disabled setting, or expiry. No elevated
  permissions, Windows security changes, or automatic game focus.
- Every notice includes compact helpers for current Safe Mode, previous/next
  collection, KeyViewer and game reload bindings. Disabled controls are labeled
  inactive. Reload uses the discovered `d3dx.ini` binding, F10 only when configured;
  an unknown binding yields an explicit manual-reload instruction.
- Distinguish collection/disk commit, runtime publication pending/failure, and a
  reload command being sent. Never claim confirmed game rendering merely because
  files were written or input was sent. Empty presets, no-op state and failures
  must not become false collection-switch successes.

## Design and minimal structure

Reading this as a transient status surface for mod-management players, using the
existing technical Workbench language: ENERGY 2 / RHYTHM 2 / MOTION 1. Follow
`design.md`, semantic tokens, existing sans type, modest radius and readable
contrast. One accent belongs to the new collection; secondary text explains its
origin. The transient surface's elevation separates feedback from the active
page; opacity preserves the game view without dimming the text itself.

- Backend owns typed action outcomes, unique notice identity, original game/root
  scope, monotonic/latest ownership and expiry. One receipt feeds app and native
  feedback; optional presentation failure must not undo a committed mod change.
- Shell subscribes once with cleanup, keeps the newest owned notice and consumes
  current runtime status by game/generation. Older terminal events/timers cannot
  replace or dismiss a successor. It renders outside route content and below
  blocking recovery/file-in-use dialogs, without focus trapping.
- Use a dedicated lightweight bundled notification entry for the optional native
  webview, not the full App bootstrap or a duplicate product shell. Keep its
  capability restricted to notice snapshot/readiness and event listening; no
  library mutation, filesystem, updater, browser or global-shortcut commands.
- Render names as text, never HTML. Bound layout/content, wrap long Unicode names,
  honor reduced motion and use EN/ID/ZH translations. The native overlay has no
  interactive controls; the app notice has a working close control and keyboard
  dismissal that does not intercept existing modal input.

## Documentation checked

- Installed Rust Tauri is 2.11.5. Context7 resolved `/tauri-apps/tauri`; checked
  its window builder and event/capability contracts against the pinned
  [official Rust API](https://docs.rs/tauri/2.11.5/tauri/webview/struct.WebviewWindowBuilder.html).
  `focused(false)` is distinct from `focusable(false)`; both matter. Transparency,
  always-on-top and skip-taskbar are native window properties, not CSS substitutes.
- [Windows extended styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles)
  document nonactivation and topmost behavior. Click-through must be verified
  through the actual native window, not inferred from `pointer-events: none`.
- [Microsoft's fullscreen explanation](https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/)
  distinguishes Borderless/Fullscreen Optimizations from exclusive presentation.
  A desktop notification cannot guarantee visibility over exclusive fullscreen;
  reuse installed 3DMigoto for that surface and report unsupported profiles.

## Implementation and risk-based validation

1. Typed outcome/notice tests first: committed collection/safe transitions, empty
   presets, default-off/backward settings loading, original scope and expiry,
   latest-wins timers, foreground/setting guards and failure isolation.
2. Add shell feedback and independent Settings control; test Home/other route,
   old/new hierarchy, current/disabled hints, long names, close/expiry,
   listener teardown and runtime-event ordering.
3. Extend the existing generated 3DMigoto status with bounded actual control
   hints (maximum 10 lines and 4 KiB), preserving independent toggle behavior.
4. Add lightweight native renderer and least-privilege capability; test native
   creation/expiry, no focus transfer, correct monitor placement and input pass
   through on owned helper fixtures. Never bind or mutate the real library.
5. Run relevant Rust/frontend tests, actual Specta export, command registry,
   TypeScript, scoped lint/architecture/format, build and native E2E with private
   profile/hash guards. Preserve startup bootstrap guard. Independent read-only
   review checks cross-module state, IPC and OS-window boundaries.
6. Document passed/failed/unexecuted cases. No installed-app replacement, public
   release, migration, dependency installation, commit or push is included.

## Authorized installed update follow-up (2026-10-11)

The user subsequently requested updating the installed executable, then chose to
defer direct game validation and use alternative checks. Coordinate Cargo/dist
with the startup/cleanup owners; wait for source-complete and the fixture release
handoff. Build the normal production profile without bundles or audit features,
verify the emitted notice entry and unchanged frozen inputs, and replace only
the installed executable after a hash-verified installation/app-data backup.
Retain the old executable for rollback and prove all other installed/app-data
files unchanged. Do not reset or launch the production library for fixture tests.
Report native fixture evidence separately from unverified real 3DMigoto,
click-through hardware, multi-monitor/DPI visuals and exclusive fullscreen.
