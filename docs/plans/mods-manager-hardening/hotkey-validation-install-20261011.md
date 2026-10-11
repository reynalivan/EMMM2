# Native hotkey completion and local installed-app update

User requested completion on 2026-10-11 (Asia/Jakarta).

## Ownership and scope

- Continue the fork's reviewed hotkey and watch-mode cache fixes without duplicating edits.
- Coordinate source/compiler/native/GUI access with the startup and cleanup chats.
- Use fresh private native E2E builds and owned synthetic Mods; never reset the production DB.
- Real Genshin is the input foreground source only. No automatic focus transfer, real mod toggle, loader change, elevation or security-setting change.
- Update the existing local installation only after relevant validation; no GitHub release, version bump, signing-key change, commit or push.

## Acceptance sequence

1. Repeat the complete native global-hotkey spec after the watcher patch. Require successful Safe Mode, both preset directions/wrap, stopped-game cycling, background overlay refusal, five complete focused replays, disabled behavior and eight snapshot/sixteen bulk cycles. Retain build hash, request/receipt identities and owned cleanup.
2. Run the supported Genshin-foreground mode with a live, uniquely identified game window. Assert foreground continuity plus synthetic disk/DB outcomes. Do not count skipped fixture-only overlay/disabled tests as Genshin coverage.
3. Check the installed executable path/profile and active processes. Build a production-profile candidate from the validated frozen source; never install an E2E executable. Preserve a recoverable installed binary/resources backup and offline production data/configuration backup before replacing anything. Refuse replacement while that exact installation is running.
4. Verify installed file hashes against the candidate and retain rollback paths. A production launch may perform ordinary startup/recovery; do not run fixture reset or synthetic mutations against it. Report installed-file verification separately from real-library behavior.

## Explicit limitations

Computer Use provides complete chord presses, not separate physical key-down/key-up or a held-key duration. Physical held-modifier tests need a human operator and must not be inferred from simulated chords or unit tests. Actual 3DMigoto rendering is separate from global dispatch and remains unclaimed unless observed directly.

## Preparation evidence

- Installed executable: `C:\Users\yusri\AppData\Local\EMMM\emmm.exe`, SHA-256 `1F20BC9155F0BCC90F483A778D40F0548B5A564B0B4015B929D3DE36A618EAA8`; existing signature status NotSigned.
- Offline installation and complete Roaming app-data backup: `C:\Users\yusri\AppData\Local\EMMM-backups\20261011-hotkey-27cb5d7d-a3e6-42d8-8501-c41537a5b1ba`. All 170 copied files matched their source hashes. No installed process was running and no installed file was changed.
- Human confirmed Genshin ready; passive Computer Use capture showed the actual game world. Genshin PID 20796 and returned executable window `D:\Games\Genshin Impact\GenshinImpact.exe` were observed. Revalidate both before input; do not reuse a dead process/window.
- Actual-game foreground validation is prioritized before the regular receiver run to use the user's ready game session. All synthetic-library and production-build boundaries remain unchanged.

## Native results before the concurrent Settings change

- M (actual Genshin, synthetic input): one Safe Mode chord had no matching action log or disk effect. Two later input requests were explicitly failed without additional keys. Bulk stress passed eight captures/sixteen toggles. Overall one pass/three failures/two intentional mode skips; both helpers exited 0 and the owned fixture was removed. This is not physical-key evidence or a watcher regression.
- N (ordinary receivers): five cases passed: preset directions and both wraps, background overlay refusal/five complete focused replays, stopped-game cycling, disabled keys, and snapshot/bulk stress. Safe Mode's operator prerequisite failed while Windows refused private-window activation with Genshin foreground; no Safe Mode chord was sent in that case. Human subsequently minimized Genshin. Both helpers exited 0 and the fixture was removed.
- O (only Safe Mode rerun): one case passed. Native assertions verified minimized EMMM and both disk/DB/settings transitions while the other receiver kept foreground. Both helpers exited 0 and the fixture was removed. Thus all six scoped ordinary-fixture cases passed across N/O; N itself is not a wholly passing run.
- P (physical keyboard, human-confirmed Borderless): Safe Mode ON/OFF passed with Genshin PID 20796 continuously foreground. Preset validation timed out; human clarified F6 was pressed repeatedly instead of requested F8. Logs confirm additional Safe Mode toggles, not a wrong F8 action. Both helpers exited 0 and the fixture was removed; stale requests must not be reused.
- Evidence directories/logs: `.audit-output/global-hotkeys-native-20261011-{m-genshin,n-fixture,o-safe,p-physical-genshin}`. Per-case private logs and outcomes retain failures without logging complete settings or credentials.
- Ordinary preset/bulk operations no longer encountered the runtime-sibling identity-cache failure. Transient runtime publication failures appeared during rapid prefix changes; N finished with a successful current Full publication. Disk/DB success alone is not a rendering certificate.

## Source coordination and final game coverage

- Another authorized chat completed seven Settings/conflict/theme/updater fixes after P and handed off frozen source: 89 frontend tests, 26 Rust tests, strict all-targets Clippy with `readonly-audit`, types, lint and architecture checks passed. Six distinct native Settings cases passed across focused fresh runs; the initial two navigation-precondition failures remain recorded. Older executables alone do not certify this newer source.
- Q (fresh physical Borderless presets): independent native executable/title/PID/birth proof and receipts verified Next, Previous, Next-again and Next-third. A raw next-wrap action also reached the expected collection, but its operator receipt timed out; the complete case FAILED and previous-wrap was not run. Both helpers exited 0 and the owned native tree/fixture were cleaned. Do not report Q as a passing suite.
- R (fresh physical Fullscreen, mode human-confirmed): TWO complete cases passed, zero failures. Safe Mode ON/OFF plus Next, Previous, Next-again, Next-third, next-wrap and previous-wrap all passed native disk/DB/settings assertions with fresh physical-input receipts, native process identity and sampled foreground continuity. Genshin PID 20796 remained the actual input foreground source; only isolated synthetic Mods changed. Extra user F6 presses are retained in logs, not hidden. Both helpers exited 0; owned native/driver processes and one fixture were cleaned; the passive operator monitor stopped.
- R evidence: `.audit-output/global-hotkeys-native-20261011-r-fullscreen/` and its sibling `.log`. Q evidence: `.audit-output/global-hotkeys-native-20261011-q-borderless-presets/` and its sibling `.log`.
- The R snapshot and production-build preflight both matched all 1546 source inputs. Relative to Q, only the independently added read-only `scripts/check-bootstrap-bundle.mjs` changed the input set; no application/config/Vite behavior drift occurred. Other chats remain source/compiler/native/dist-frozen until installed-file verification finishes, per the user's explicit priority instruction.
- The verified 170-file offline backup matched the original installation and production app-data immediately before replacement. The fresh ordinary production `tauri build --no-bundle` passed; no E2E/readonly-audit profile was used.

## Installed update result

- Updated only `C:\Users\yusri\AppData\Local\EMMM\emmm.exe` at `2026-10-11T11:38:24Z` by staging the fresh candidate, verifying hashes and inactive native processes, then atomic `File.Replace` with a separate old-executable backup.
- Candidate and installed SHA-256: `CAB10B03BF9A6ACE76E7EB2F2455A39B12846BEDF33AD419EA10A601FBF6FA89`; size 65,915,392 bytes. Candidate: `E:\code\projects\EMMM2\src-tauri\target\release\emmm.exe`. Normal production identifier: `com.reynalivan.emmm`.
- Atomic old-executable backup: `C:\Users\yusri\AppData\Local\EMMM-backups\20261011-hotkey-27cb5d7d-a3e6-42d8-8501-c41537a5b1ba\emmm-before-swap.exe`, verified against original SHA-256 `1F20BC9155F0BCC90F483A778D40F0548B5A564B0B4015B929D3DE36A618EAA8`. The original full installation and offline app-data backup remain in the same backup root.
- All 169 other installed/app-data files remain hash-identical to their offline backups; no fixture reset, production DB/config change or real-library mutation occurred. Installed resources already match source resources; no resource replacement was needed. No native app/driver process remained active.
- The installed app was not launched after replacement. Reopen EMMM normally to use the updated executable; installed-file verification is not a production-library runtime smoke test. Package version remains 0.1.0; both old and new executables are NotSigned. No public release, installer, version bump or signing-key change was performed.
- Production build passes frontend types/Vite and optimized Rust compilation. Rust emits five existing unused-code/field warnings; a clean ordinary production warning baseline is not claimed. This does not invalidate the previously reported strict all-targets `readonly-audit` Clippy result, which uses a different feature/target surface.
- Final post-install checks passed: E2E TypeScript, strict scoped ESLint, four identity unit tests, scoped Prettier, repository `git diff --check`, and all 1546 R source-input hashes. No source changes entered between physical Fullscreen validation, the ordinary production build and installed-file verification.
- For a rollback, first close EMMM and preserve any newer app-data; the verified old executable is recoverable. Do not automatically restore the old DB/WAL/config over newer production activity or infer backward compatibility from a binary backup.

## Physical-harness review corrections

- Physical game bindings use reachable F6/F8/F9/F10; operator waits are five minutes, ordinary automation retains three minutes. Before-observation age is bounded by the physical wait only; after remains fresh within fifteen seconds. Identity, token and sampled foreground continuity are still required.
- Passive receivers optionally observe only the declared foreground Genshin PID using read-only limited process queries. Executable/title plus lossless creation FILETIME must match before/after. Unavailable process information rejects certification without privilege elevation; unrelated foreground metadata is not collected.
- Expiry/cancellation are token-bound; failed physical input/cases stop remaining physical scenarios. Diagnostic reads are independent and bounded; their failures are retained without replacing the original case failure.
- Four assertion-layer tests pass after a missing-module RED. E2E types, strict scoped lint, formatter checks and C# warnings-as-errors compilation pass; independent corrected-scope review has no actionable finding. Q/R execute the enriched Win32 observation path; Q retains failure/cancellation evidence and R passes both full cases. P predates these corrections.
- APIs were checked via Context7 and the official [process-image reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew), [process-times reference](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes) and [window-caption reference](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextw).
