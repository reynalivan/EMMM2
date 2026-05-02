# Runtime sync gap cleanup

## Context

Phase 1 runtime sync sudah masuk, tetapi masih ada dual-writer watcher, event result belum scoped per game, invalidasi collections salah key, dan beberapa helper legacy masih tertinggal.

## Changes

- Watcher lifecycle sekarang hanya batching event lalu mendelegasikan sync ke gateway `runtime_sync`; mutasi DB batch dipindah ke service runtime sync.
- Menambahkan state machine runtime sync yang merge request per game dan menjalankan trailing rerun sampai pending work habis.
- Contract `RefreshRuntimeResult` sekarang membawa `game_id`, `reason`, dan `runtime_file_changed`.
- Runtime reconcile sekarang menghitung `cleared_selection_paths` dari diff object roots sebelum/sesudah reconcile.
- Frontend runtime sync sekarang:
  - filter event berdasarkan `game_id`,
  - invalidate `collectionKeys.all`,
  - tidak restart watcher saat `safeMode` berubah.
- `info.json` sekarang ikut dianggap file watcher yang relevan.
- `quickImport` dihapus dari frontend service/test karena flow utama sudah pindah ke `refreshRuntimeState`.

## Impacted Files

- `src-tauri/src/services/runtime_sync/mod.rs` (added)
- `src-tauri/src/services/runtime_sync/types.rs` (added)
- `src-tauri/src/services/runtime_sync/path_classifier.rs` (added)
- `src-tauri/src/services/runtime_sync/watcher_batch.rs` (added)
- `src-tauri/src/services/runtime_sync/reconcile.rs` (added)
- `src-tauri/src/services/runtime_sync/orchestrator.rs` (added)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/services/scanner/watcher/mod.rs` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/file-watcher/pathUtils.ts` (added)
- `src/features/file-watcher/hooks.test.ts` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/lib/bindings.ts` (modified)
- `src/lib/services/scanService.ts` (modified)
- `src/lib/services/scanService.test.ts` (modified)
- `src/features/object-list/ObjectList.test.tsx` (modified)

## Goal

Runtime sync sekarang lebih dekat ke arsitektur single-gateway: watcher menjadi trigger, result event aman per game, collection cache tidak stale, dan burst request tidak mudah drop perubahan.

## Impact

- Drift antara watcher path dan manual/focus refresh berkurang karena semua lewat runtime sync gateway yang sama.
- `info.json` changes sekarang bisa ikut memicu runtime refresh.
- Selection object lama bisa dibersihkan saat root hilang/berubah.
- Masih ada risiko residual di runtime reconcile yang masih memakai preview commit ringan dengan empty MasterDB untuk scoped runtime scan.

## Notes

- Validasi yang dijalankan: `pnpm exec tsc --noEmit`, `cargo check --manifest-path src-tauri/Cargo.toml`, `pnpm exec vitest run src/features/file-watcher/hooks.test.ts src/lib/services/scanService.test.ts`.
- `cargo test --manifest-path src-tauri/Cargo.toml path_classifier -- --nocapture` gagal di environment ini dengan `STATUS_ENTRYPOINT_NOT_FOUND`, jadi unit test Rust baru belum bisa diverifikasi lewat test runner lokal.
