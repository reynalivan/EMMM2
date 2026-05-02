# Deep Match Scanner E2E cleanup

## Context

Deep Match Scanner sudah dipisah dari Disk Reconcile, tetapi masih ada gap end-to-end pada browser import, manual sync per item, dan surface FE legacy scan.

## Changes

- Browser import review sekarang mempertahankan canonical relation yang sudah tersimpan di `import_jobs`.
- Browser import placement sekarang memicu Disk Reconcile scoped setelah file dipindah, jadi projection DB/UI tidak menunggu watcher.
- Manual "Sync with DB" sekarang juga menyimpan canonical relation ke object fisik lewat command khusus `apply_object_match_cmd`.
- FE tidak lagi membangun `matched_entry_key` manual dari nama biasa; key canonical sekarang datang dari backend MasterDB payload.
- Wrapper FE generic scan lama (`startScan`, `getScanResult`) dibuang dari service/binding user-facing.
- Object row sekarang menampilkan alias canonical sebagai info sekunder tanpa mengganti nama fisik utama.
- Requirement dan flow docs disinkronkan ke boundary final Deep Match Scanner vs Disk Reconcile.

## Impacted Files

- `src-tauri/src/services/browser/import_service.rs` (modified)
- `src-tauri/src/repo/object_repo.rs` (modified)
- `src-tauri/src/commands/objects/object_cmds.rs` (modified)
- `src-tauri/src/commands/objects/tests/object_cmds_tests.rs` (modified)
- `src-tauri/src/services/scanner/master_db.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src/lib/bindings.ts` (modified)
- `src/lib/services/scanService.ts` (modified)
- `src/lib/services/scanService.test.ts` (modified)
- `src/features/object-list/objHandlersHelpers.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/locales/en/objects.json` (modified)
- `src/locales/id/objects.json` (modified)
- `src/locales/zh/objects.json` (modified)
- `.docs/flow.md` (modified)
- `.docs/requirements/req-07-object-list.md` (modified)
- `.docs/requirements/req-25-scan-engine.md` (modified)
- `.docs/requirements/req-38-auto-organizer.md` (modified)
- `.docs/requirements/req-44-discover-hub-smart-import.md` (modified)

## Goal

Deep Match Scanner sekarang konsisten sebagai enrichment + canonical relation flow, sementara identitas fisik folder/object tetap milik disk.

## Impact

- Auto Organize, archive/folder import review, dan browser import lebih konsisten memakai flow Deep Match Scanner resmi.
- Browser import selesai dengan refresh runtime yang lebih cepat dan lebih deterministik.
- Ada breaking change kecil pada FE internal API: wrapper `scanService.startScan` dan `scanService.getScanResult` tidak lagi tersedia.

## Notes

- Residual gap yang sengaja belum dipaksa: jika browser import fallback ke folder fisik `Other`, canonical relation belum di-attach ke object row bersama karena itu berisiko mengotori object `Other` yang dipakai banyak mod.
