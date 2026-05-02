# Fix Collection Preview Mod Counts

## Context

Kolom mods di collection list dan header preview masih memakai jumlah member snapshot mentah, sehingga `VariantContainer` dan branch container dihitung berlebihan dibanding tree preview yang tampil.

## Changes

- Tambahkan helper shared untuk menghitung mod yang terlihat dari preview tree.
- `VariantContainer` sekarang dihitung sebagai 1 mod.
- `ContainerFolder` parent tidak dihitung sebagai mod.
- `mod_count` per object tree sekarang memakai hasil collapse tree, bukan jumlah row snapshot mentah.
- `CollectionSummary` sekarang membawa `mod_count` ter-normalisasi untuk dipakai list/topbar/preview header.
- UI collections list, topbar dropdown, dan preview panel sekarang membaca `mod_count` baru.

## Impacted Files

- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/services/collection_preview_tree.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src/types/collection.ts` (modified)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src/features/collections/hooks/useCollections.test.ts` (modified)

## Goal

Angka mod yang tampil di list dan preview sekarang cocok dengan semantics tree preview yang dilihat user.

## Impact

- Count di collection list, topbar dropdown, preview header, dan badge object tree jadi konsisten.
- Tidak ada schema migration.
- Runtime Vitest di environment ini sempat terblokir `esbuild spawn EPERM`, jadi verifikasi frontend final mengandalkan `tsc --noEmit`; compile Rust test berhasil.
