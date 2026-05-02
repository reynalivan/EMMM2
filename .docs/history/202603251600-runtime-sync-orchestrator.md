# Runtime sync orchestrator

## Context

ObjectList masih bisa stale terhadap state filesystem, sementara FolderGrid sudah lebih disk-first. Trigger onboarding, masuk halaman mods, focus window, dan watcher belum lewat satu pipeline reconcile yang sama.

## Changes

- Menambahkan backend `refresh_runtime_state` sebagai gateway sinkronisasi runtime disk -> DB dengan reason enum dan hasil refresh terstruktur.
- Menambahkan service `runtime_sync` untuk klasifikasi root yang berubah, scoped reconcile, lock per game, dan side effect runtime seperti dirty-state collections, refresh overlay, dan invalidasi thumbnail roots.
- Mengubah watcher lifecycle agar setelah batch sukses ia mengirim event `runtime_sync:result` dari hasil finalisasi runtime, bukan hanya mengandalkan invalidate query frontend.
- Mengganti coordinator frontend lama dengan runtime sync coordinator yang:
  - start/stop watcher,
  - refresh saat masuk view `mods`,
  - refresh saat window focus,
  - consume `runtime_sync:result`,
  - invalidate cache setelah reconcile backend selesai.
- Mengubah manual background sync object list dan finalisasi onboarding agar memanggil `refreshRuntimeState` daripada `quickImport`/watcher retry.
- Menambahkan bookkeeping store untuk timestamp sync dan dirty marker per game.

## Impacted Files

- `src-tauri/src/services/runtime_sync/types.rs` (added)
- `src-tauri/src/services/runtime_sync/reconcile.rs` (added)
- `src-tauri/src/services/runtime_sync/orchestrator.rs` (added)
- `src-tauri/src/services/runtime_sync/mod.rs` (added)
- `src-tauri/src/commands/scanner/runtime_sync_cmds.rs` (added)
- `src-tauri/src/services/mod.rs` (modified)
- `src-tauri/src/commands/scanner/mod.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/lib/bindings.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/file-watcher/ExternalChangeHandler.tsx` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/onboarding/WelcomeScreen.tsx` (modified)

## Goal

Runtime sync sekarang dipicu dari satu jalur yang sama sehingga projection DB, ObjectList, collection dirty-state, overlay, dan thumbnail refresh bergerak sesudah reconcile filesystem selesai.

## Impact

- Masuk halaman mods, onboarding selesai, refocus window, dan watcher batch sekarang bisa memperkecil drift antara disk dan UI.
- Watcher frontend tidak lagi menjadi sumber kebenaran status.
- Resource tetap efisien karena scoped reconcile watcher memakai root yang berubah bila tersedia.
- Transisi masih parsial: watcher DB batch lama tetap dipakai untuk heal rename/path update, lalu hasilnya dinormalisasi lewat `runtime_sync:result`.

## Notes

- Validasi yang dijalankan: `pnpm exec tsc --noEmit`, `pnpm exec vitest run src/features/file-watcher/hooks.test.ts`, `cargo check --manifest-path src-tauri/Cargo.toml`.
