# Validate physical global hotkeys

## Context

Complete reviewed watcher/replay native coverage, actual Genshin input and the
requested installed-app update without racing concurrent source owners.

## Changes

- Add accessible physical-game bindings, explicit source/freshness/expiry,
  cancellation and fail-stop physical scenarios.
- Independently retain bounded per-case diagnostics without full settings.
- Add passive foreground process/executable/title/birth proof and four identity
  assertion tests. No hooks, input injection or privilege elevation in receivers.
- Verify an offline installation/app-data backup, 170 matching file hashes.

## Impacted files

- `tests/e2e/specs/global-hotkeys-native.e2e.ts`
- `tests/e2e/support/hotkey-input-window.cs`
- `tests/e2e/support/hotkeyIdentity.ts` (added)
- `tests/e2e/support/hotkeyIdentity.test.ts` (added)
- `docs/plans/mods-manager-hardening/hotkey-validation-install-20261011.md` (added)
- This history; ignored evidence/config/compiler outputs in `.audit-output/`.

## Verification and limits

N/O jointly pass six ordinary native cases, including five complete overlay
replays, both preset wraps, disabled/stopped-game behavior and 8 captures/16 bulk
toggles. P physically passes Safe Mode ON/OFF in human-confirmed Borderless;
preset timed out after the human accidentally repeated F6 instead of F8.
All owned helpers/processes/fixtures were cleaned. Four identity tests, types,
strict scoped lint, formatter and C# warnings-as-errors pass; corrected review
has no finding. Q executes the passive identity path and verifies Borderless
Next/Previous steps, but the whole case fails at an operator wrap-receipt timeout.
R physically passes both complete Fullscreen cases: Safe Mode ON/OFF and all six
preset directions/wrap steps, with native identity, receipts, sampled focus and
disk/DB/settings assertions. Both receivers exit 0, the native/driver tree and
one fixture are cleaned, and the passive monitor is stopped. No synthetic
keyboard input is used for R.

## Goal and impact

No production source, real library, dependency or schema was changed here.
Concurrent Settings fixes are handed off frozen with relevant checks; all 1546
R/production build inputs match. The user prioritized this task and other chats
hold source/compiler/native/dist access until installed-file verification.
The complete 170-file offline backup matched before replacement. A fresh normal
production build passed and only the installed executable was replaced atomically
at 2026-10-11T11:38:24Z. Candidate/installed SHA-256 is
`CAB10B03BF9A6ACE76E7EB2F2455A39B12846BEDF33AD419EA10A601FBF6FA89`;
the old executable remains hash-verified in the backup root recorded in the plan.
All other 169 installation/app-data files remain unchanged. No production launch
was performed; the user can reopen EMMM normally. Five ordinary production Rust
unused-code/field warnings remain recorded, not suppressed. Actual loader
rendering, held-modifier behavior and a real-library runtime smoke remain
separate, unverified claims.
