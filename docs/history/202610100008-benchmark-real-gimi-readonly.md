# Benchmark real GIMI with read-only native isolation

## Context

User requested a native Windows startup audit against the real GIMI library,
with no source-file mutation/deletion and restored isolated configuration.

## Changes

- Add a private debug-only audit profile that requires its root flag on every
  launch, rejects unsafe recovery/configuration, gates mutation IPC and skips
  the entire runtime publication/reload/cleanup pipeline.
- Reuse native driver lifecycle with a private executable/app-data identifier;
  seed and restore its database offline with canonical overlap/link checks.
- Add whole-library SHA256/metadata manifests, final metadata verification and
  native restart/frame measurements. Raw backups and manifests are ignored.
- Record measured costs, boundaries and follow-up recommendations.

## Impacted files

- `.gitignore`; `scripts/audit-library-manifest.ps1`.
- `wdio.conf.ts`; `wdio.readonly-audit.conf.ts`.
- `src-tauri/tauri.readonly-audit.conf.json`; `src-tauri/AGENT.md`.
- `src-tauri/src/lib.rs`.
- `src-tauri/src/modules/system/application/app/mod.rs`.
- `src-tauri/src/modules/system/application/app/readonly_audit.rs`.
- `src-tauri/src/modules/system/application/app/bootstrap.rs`.
- `src-tauri/src/modules/system/application/app/post_apply.rs`.
- `tests/e2e/support/readonlyAudit.ts`.
- `tests/e2e/specs/startup-real-library-readonly.e2e.ts`.
- `docs/plans/startup-performance/real-gimi-readonly-20261010.md`.
- `docs/plans/startup-performance/validation.md`.
- This history file.

## Validation and impact

Three actual native restarts passed on 632 GB / 375,099 files. Home became
visible in 0.9–1.3 seconds, core Ready in 4.4–4.9 seconds and keybindings settled
in 6.5–12.0 seconds. One startup baseline/watcher per restart. Configuration
restoration completed with zero real-library bindings. All 375,099 file SHA256
digests and full before/after manifests match. Final integrity evidence is
recorded in the audit report.

Six guard tests, 50 post-apply tests (two ignored), nine driver lifecycle tests,
audit-scoped TypeScript, focused ESLint, native build/check and strict library
Clippy pass. Full E2E TypeScript remains limited by an unrelated concurrent
Window-cast error. Safety review found no remaining actionable blocker before
the real run. Owned fixture checks detect content and metadata changes.

Normal startup behavior and the installed executable are unchanged by this
audit harness. OS-cold startup and runtime KeyViewer publication are unmeasured.
