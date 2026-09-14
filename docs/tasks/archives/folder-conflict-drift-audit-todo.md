# Checklist Anti-Drift Reconcile dan Folder Conflict

## Collection integrity

- [x] T1: Tambahkan collection reference API game-scoped dan wildcard-safe.
- [x] T1A: Migrasikan collection membership ke logical keys + nullable runtime bindings.
- [x] T1B: Pertahankan missing state dan rebind saat object/mod kembali.
- [x] T2: Migrasikan seluruh Disk Reconcile caller.
- [x] T3: Migrasikan mutation caller dan lakukan transactional child impact sebelum parent delete.
- [x] Checkpoint A: cross-game, wildcard, parent-delete tests hijau.

## Snapshot, locking, dan watcher

- [x] T4: Gunakan satu full snapshot koheren per reconcile.
- [x] T5: Satukan mutation dan reconcile dalam per-game lease.
- [x] T6: Tegakkan recovery contract untuk setiap suppression guard.
- [x] T7: Tambahkan watcher backpressure, overflow repair, dan bounded toast dedupe.
- [x] Checkpoint B: concurrency/event-storm tests hijau tanpa silent drift.

## Durable identity dan conflict commit

- [x] Approval: migration aditif filesystem identity dengan fallback `NeedsRenameConfirmation`.
- [x] T8: Simpan optional filesystem identity.
- [x] T9: Heal nested/offline rename secara konservatif; missing/ambiguous identity meminta keputusan pengguna.
- [x] T9A: Gate startup/game activation pada full recovery reconcile.
- [x] T9B: Stage/finalize collection references sebagai batch untuk mod swap dan parent swap.
- [x] T9C: Simpan terminal recovery per activation; cegah settings/workspace/cache hydration mendahului full disk scan.
- [x] T10: Commit selected conflict group secara atomik.
- [x] T11: Tambahkan journal/recovery two-phase rename.
- [x] Checkpoint C: restart, lost-event, partial-group, stale-resolution, dan crash tests hijau.

## UI dan final verification

- [x] T12: Jadikan details bounded/cancellable dengan loading-error-retry.
- [x] T13: Satukan report state, legacy conflict, relative path contract, dan rename-confirmation report per game.
- [x] T14: Jalankan acceptance matrix dan full verification.
- [x] `cargo fmt --check`.
- [x] Targeted Rust tests.
- [x] `cargo test` (745 passed, 1 ignored).
- [x] `pnpm test -- --run` (710 passed, 1 skipped).
- [x] `pnpm lint` (0 errors; warnings non-blocking dicatat oleh ESLint).
- [x] `pnpm i18n:lint`.
- [x] `pnpm build`.
- [x] Automated matrix: Rust integration mencakup create/delete/toggle/reconnect/bulk/concurrency; targeted E2E mencakup conflict queue dan nested rename saat watcher offline/startup recovery.
- [ ] Manual Explorer/Open Folder/Recycle Bin restore smoke check.
