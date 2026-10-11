# Main publication and Windows installer validation

12 October 2026, Asia/Jakarta. The user authorized publishing all completed
application, test and documentation changes, excluding local audit data,
screenshots, databases and secrets. No tag, GitHub Release or installation is
included.

## Source checks

- Full frontend: 243 files, 1,517 passed, one existing skip. The final direct-Node
  raw log contains zero React/act/key/DOM, missing-mock or undefined-query warnings.
  All 18 remaining stderr contexts intentionally exercise failures.
- Full default Rust library: 1,855 passed, zero failed, 40 intentionally ignored
  manual benchmarks/helpers. Earlier two failed drift checks were corrected only
  in the INI golden and frozen census reference; production behavior is unchanged.
- Actual Specta/registry export passed; bindings remain byte-identical at
  `a2790111d5d4bfea4948bc8989eaee7bfde9f17cff64981a1c3c68015745917f`.
- Coordinated full lint/architecture, app/E2E types, strict Clippy, authored-project
  formatter and diff checks passed. Publication/packaging boundary review found
  no actionable blocker. Its scope is not a new whole-repository security audit.
- The final private native suite passed 3/3. Its certificate retains five explicit
  test-only deltas and one HTML whitespace-only delta, not a false whole-tree
  zero-drift claim. See the settings/shell validation for exact proofs and limits.

## Installer creation

Normal production command: `node scripts/tauri.mjs build --bundles msi,nsis --ci`.
No private profile, audit feature, signing bypass or version-mismatch bypass was
used. Version remains synchronized at 0.1.0.

Frontend and optimized Rust compilation succeeded; Rust took 2m24s. WiX/candle/light
created the MSI and makensis created the NSIS EXE. The overall command exited 1
afterward because the existing updater private key could not be decrypted with
the available password. No password was guessed, key rotated, or check disabled.

| Artifact                 | Bytes      | SHA-256                                                          |
| ------------------------ | ---------- | ---------------------------------------------------------------- |
| EMMM_0.1.0_x64_en-US.msi | 24,035,328 | 1378c7afae97e99a362223e407d8f60db3b8f75f3dfa79d2369307ff8bce2c73 |
| EMMM_0.1.0_x64-setup.exe | 16,825,270 | 5304980876eff8f1febe7d2e38418b73d0747ca4cebdb71f5878344b59dca3ba |

Both files are local installers without Authenticode publisher signatures.
Updater `.sig` files are absent, so signature/tamper verification on these
installers was not executed and they are not ready for signed updater delivery.
The compiled verification helper uses the application's existing Minisign
verifier; compilation and read-only review are not signature verification.

The MSI database was opened read-only and reports EMMM, version 0.1.0 and
manufacturer reynalivan. Neither installer was executed. Previously existing
installers were copied and hash-verified in the ignored evidence directory.

## Build provenance and preservation

All 1,585 production-build inputs matched after packaging. Eight additional
resource/environment-file hashes matched; no environment values were disclosed.
The actual notice HTML was emitted. The final executable is
`3a2ca755662adcc99e424985be42387262b9d945cdcca7842dd7af2d28c5fe7a`;
Tauri patches bundle-type metadata for MSI and NSIS, so this is the final NSIS
build executable, not a claim that both installer payloads are byte-identical.

Evidence is retained in `.audit-output/release-main-20261011/`: raw test/build
logs, source verification, installer manifest/checksums and previous installers.
Commit/remote identity is recorded separately in its publication receipt.

Installed EMMM and the actual mod library were not launched, replaced or reset by
this packaging task. Actual installation, real 3DMigoto rendering, hardware
click-through, multi-monitor/DPI visuals and exclusive-fullscreen visibility are
not certified. Signing with the existing protected key remains a user-controlled
follow-up; no secret should be sent through chat.
