# Checklist Source Recovery dan Mutation Anti-Drift

## Baseline dan approval

- [x] Audit current `SourceUnavailable` dialog/banner implementation
- [x] Audit source directory change flow
- [x] Audit auto-organize, auto-category, thumbnail, preview, info.json, dan INI mutation paths
- [x] User menyetujui matching auto-sync, empty confirmation, dan hard confirmation untuk different library
- [x] Execution plan dibuat tanpa menimpa `tasks/plan.md`/`tasks/todo.md` pekerjaan lain

## Backend source recovery

- [x] T1 Candidate inspection contract dan classifier
- [x] T2 Stale-safe directory apply dan config rollback
- [x] T3 Watcher generation untuk root replacement
- [x] Checkpoint A targeted Rust/watcher tests

## Source recovery UI

- [x] T4 Source recovery dialog state machine
- [x] T5 Generated contract, permissions, dan i18n EN/ID/ZH
- [x] Targeted dialog/store/frontend tests

## Metadata stability

- [x] T6 Category persistence dan atomic auto-category
- [x] T7 Atomic auto-recognize metadata
- [x] T8 Metadata/info.json mutation convergence
- [x] Checkpoint B projection/rollback tests

## Filesystem CRUD convergence

- [x] T9 Thumbnail CRUD hardening
- [x] T10 Preview image dan INI CRUD convergence
- [x] T11 Auto-organize and collection regression gate

## End-to-end dan final verification

- [x] T12 Source recovery/external mutation E2E
- [ ] Manual Windows picker/Explorer/Trash/Recycle Bin check
- [x] T13 Filesystem writer architecture audit
- [x] T14 Final review gaps: collection-bearing identity swaps dan per-game activation/startup recovery gate
- [x] Targeted Rust tests
- [x] Targeted frontend tests
- [x] `cargo test`
- [x] `pnpm test -- --run`
- [x] `pnpm lint`
- [x] `pnpm i18n:lint`
- [x] `pnpm build`
- [x] `cargo fmt --check`
- [x] `git diff --check`
- [x] Final history document
