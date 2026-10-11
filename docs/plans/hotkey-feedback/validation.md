# Hotkey feedback validation

2026-10-11, Asia/Jakarta. The scoped native fixture flow passed; limitations below
remain explicit.

## Installed update follow-up

The user authorized updating the installed application, then explicitly deferred
direct Genshin validation in favor of alternative checks. On 2026-10-11 the
normal production executable was built with `node scripts/tauri.mjs build
--no-bundle`, without private configuration or audit features, and installed by
atomic executable replacement. Production was not launched or reset.

- Fresh follow-up checks: 43 frontend tests in six focused files, three Node
  contracts and the actual write-free bootstrap/notice bundle guard passed.
- The coordinated startup/cleanup capture passed all 15 native release cases
  across five specs, using the private fixture profile. Its 1577 source inputs
  matched after execution, owned processes shut down, fixtures were cleaned, and
  installed DB hash remained unchanged. This covers onboarding, sync/recovery,
  restart, preview and Dashboard, not new direct-game overlay certification.
- Production build exited 0; Rust release compilation took 2m49s. Five existing
  library warnings remain; this does not establish global Clippy cleanliness.
  `dist/hotkey-notice.html` was emitted. All 1577 frozen source inputs still
  matched after production build.
- Installed/source executable SHA-256:
  `521a04c3f577efaa88bc351d79f0841344824a555f3251557acb547ce8a78077`.
  Version remains 0.1.0; Authenticode remains NotSigned, as before. This is an
  authorized local update, not a signed public updater release.
- Hash-verified backup of 9 installation files and 161 app-data files:
  `C:/Users/yusri/AppData/Local/EMMM-backups/20261011-notice-a4665a37-0c64-4eaa-a6dd-756ff7b2cb87`.
  The original executable is also retained as `emmm-before-swap.exe`; no automatic
  rollback or app-data restoration was performed. All 169 non-executable files
  were verified unchanged. Seven shipped resources match the installed versions.
- Evidence: `.audit-output/hotkey-installed-update-20261011/` contains the backup
  inventory, installed receipt, production log and zero-drift source/binary proof.
  Coordinated native evidence is in
  `.audit-output/final-performance-audit-20261011/native-release/`.

The executable is now updated, superseding the historical installed-hash statement
in run S below. Real 3DMigoto rendering, actual mouse pass-through,
multi-monitor/DPI visual placement and exclusive-fullscreen visibility remain
unverified. User preferences were preserved; the native game notice remains
default-off. No real-library mutation, dependency change, public release, commit
or push was performed by this follow-up.

## Scope and behavior

- The shell notice is outside route content, including Home, Settings and other
  pages. It shows old/new collection context, Safe Mode ON/OFF, and five actual
  control helpers: Safe Mode, previous/next collection, in-game KeyViewer and
  discovered reload. The two optional game surfaces retain separate settings.
- The native notice is default-off and lazy. It cannot activate or consume mouse
  input; original game/root identity, scope, foreground and expiry are checked
  before show. Fatal startup and failed hide have token-bound retirement paths.
- Game rendering is never inferred from disk commit or a sent reload command.
  Exclusive fullscreen visibility is not guaranteed. Native custom themes use a
  readable Onyx fallback; literal Light uses the built-in light tokens.

## Executed checks

- Frontend: 60 tests passed across nine targeted files (notice presentation,
  ownership, snapshot/event/expiry ordering, renderer recovery, settings controls,
  browser visibility/focus, App and runtime listener races).
- Backend after final ownership/retirement fixes: 8 controller tests, 49 hotkey
  tests, 7 runtime queue tests and 4 status-generation tests passed, zero failures.
  The concurrent registry owner also ran actual Specta export/registry: 2 passed.
- App and E2E TypeScript passed against the regenerated contract. Scoped ESLint
  and architectural lint passed. C# receiver compilation with warnings as errors
  passed; the compiled probe was not executed or installed.
- Three Node bootstrap contracts passed. The real write-free bundle guard failed
  when the notice entry was absent, then passed after explicit main/notice inputs
  were restored. It now requires actual `hotkey-notice.html` emission, selects
  `index.html` explicitly, and confirms no eager Recharts while retaining charts.
- Built-in body-text contrast was calculated: Onyx 13.32:1, Light 16.30:1.
  The large primary collection title clears 3:1 in both themes. Conservative
  native 88%-opacity backgrounds also clear the relevant thresholds: conservative
  Onyx body/title 9.19:1 / 3.08:1, Light body/title 12.45:1 / 3.60:1. Primary titles
  are large text, not normal text; the checker correctly reports their normal-text
  threshold as failing and their applicable large-text threshold as passing.
  Custom user themes are not certified by those built-in measurements.
- Read-only independent feature review has no remaining actionable finding.
  Accepted fixes have RED/GREEN regression evidence; unrelated dirty changes were
  excluded. Review did not substitute for native execution.

## Concurrency and drift

Startup's automatic chunk splitting is preserved. All source owners agreed to
freeze after their bounded compiler/registry handoffs. A Vite multi-entry block
disappeared during shared work; no owner identified its source. The omission was
detected, reproduced with the strengthened artifact guard, restored and verified.
No clean-baseline or zero-concurrent-change claim is made before the final source
snapshot. The new top-level HTML is included in that snapshot inventory.

## Native result S

- Fresh private debug app, production Vite build: `com.reynalivan.emmm.e2e`;
  binary SHA-256 `9f68e5817f480b84e055fa5e5cad0d31ca4c77aa7b06c0f008a7271668ea8531`.
  `hotkey-notice.html` and its lightweight entry were actually emitted. No installed
  executable or browser demo was used.
- One complete native case passed with three genuine OS shortcut inputs to owned
  receivers: Safe Mode ON from the other app with Home feedback/no native overlay;
  Safe Mode OFF with Settings feedback and opted-in native visibility; Next
  collection A All → B Safe with native notifications OFF and hotkeys still ON.
  Disk/DB/settings outcomes, old/new labels, helper bindings, topmost/undecorated
  properties, expiry/getter-null and sampled unchanged foreground passed.
- All 1572 snapshotted build inputs matched through native completion. S artifacts:
  `.audit-output/global-hotkeys-native-20261011-s-notice/`, sibling `.log`, and
  `.audit-output/hotkey-notice-source-freeze-20261011-s/build-inputs.json`.
- Helpers 77772/45464 exited 0. Owned native/driver processes stopped; one owned
  fixture was removed, then absence and zero native processes were rechecked.
  WebdriverIO recovered stale-element observations as notices replaced themselves;
  no case failed. This is not a clean-warning baseline claim.
- Installed executable remained SHA-256
  `CAB10B03BF9A6ACE76E7EB2F2455A39B12846BEDF33AD419EA10A601FBF6FA89`.
  The new feature has not been installed into that production executable.

## Remaining limits and post-S source

Actual mouse pass-through, multi-monitor/DPI visual placement and real 3DMigoto
rendering were not directly verified. Native window traits/readiness/visibility
and foreground assertions are narrower evidence. Computer Use did not return a
targetable notice window during the attempted brief capture; no screenshot-based
claim is made. Exclusive fullscreen remains unsupported as a visibility guarantee.

After S's verified shutdown, other chats resumed their authorized disjoint
performance/preview work. Their later changes are not an in-run S drift or a
certificate from S. This task additionally corrected ID/ZH global-hotkey description
parity and added a small positive-extent guard to reject empty/disjoint/overflowing
client-monitor intersections instead of wrapping a negative width to `u32`.
The boundary guard's targeted test result is recorded separately from S; S's
binary/hash remains the pre-follow-up artifact.

The current compiled Rust test executable (rebuilt at 13:28:24 UTC) listed and
passed both `game_detector::tests`, including positive, empty, disjoint and
overflowing extents. This was a test-only command, not an app launch. The two
localized description files passed format checks; 19 related presentation/Settings
tests passed after their correction. No subsequent native recertification is
claimed for those post-S changes. In total, this task ran 70 focused Rust tests
(68 before S plus these 2), without counting duplicate runs.

## Delivery checks

UI filter gates: real old/new data, one highlighted focal value, existing semantic
themes/type, no placeholder assets, no decorative loops, safe text, working close
and Escape with modal priority, reduced-motion-compatible static presentation,
explicit failures/inactive helpers, and preserved focus. Source checks, native
executed controls and computed built-in contrast are evidence; omitted hardware
and custom-theme cases are not labeled PASS.

No production app-data, real library, installed executable, public release,
dependency, schema, commit or push was changed for this feature.
