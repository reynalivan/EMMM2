# Dashboard and preview ownership fixes

The user authorized all reported fixes/cleanup on 11 October 2026, including
validation and remediation of the three pending integration candidates.

## Scope

1. Cancel obsolete Dashboard review requests; make classification wizard launches
   own their async state, errors, callbacks and busy completion.
2. Retain viewer receipt/analysis ownership and notify mounted observers on dismiss.
3. Bind clipboard/picker mutations to the original game and folder; settle both
   thumbnail and gallery caches immediately on successful image import/paste.
4. Revalidate original root/folder identity after image mutation waits and viewer
   manifest scanning; reuse the existing filesystem identity proof contract.
5. Delete unused useModInfo, Dashboard DTO fallbacks and pie gradient definitions.
6. Migrate native visible-only preview assertions to the visibility-aware IPC,
   then remove the unused legacy command through registry/permissions/Specta/demo.

## Ownership and preservation

- Dashboard worker owns Dashboard/classification frontend files and regressions.
- Preview worker owns frontend viewer/image hooks and regressions.
- Parent owns Rust image/viewer guards, native assertion migration, registry,
  generation, integration review, documentation and final validation.
- Preserve existing changes, Vite splitting, stored migrations/recovery/identity
  guards. No dependency, worktree, commit, installed app or real library change.
- Shared registry/binding and Cargo/native slots are coordinated with the existing
  hotkey notification task; source snapshot is frozen before a fresh native build.

## Verification

- Test-first deferred frontend regressions and Rust filesystem ownership cases.
- Targeted existing frontend/Rust tests, app/E2E types, scoped lint/architecture,
  formatting, command registry and actual Specta export.
- Independent review of concurrency, identity and cleanup contracts.
- Fresh isolated Windows native E2E against owned synthetic fixtures; no browser
  demo, clipboard/executable side effect on user state or production reset.
- Record exact commands, failures, snapshot integrity, teardown and coverage limits.
