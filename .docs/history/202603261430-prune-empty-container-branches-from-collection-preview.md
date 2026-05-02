# Prune Empty Container Branches From Collection Preview

## Context

Collection preview masih menampilkan `ContainerFolder` yang tidak punya terminal mod aktif, sehingga object row bisa muncul dengan `0 mods` dan hanya berisi folder kosong.

## Changes

- Menambahkan pruning rekursif di builder preview tree untuk menghapus branch `ContainerFolder` yang tidak memiliki terminal mod aktif.
- Object row sekarang tidak dikirim ke frontend jika setelah pruning tidak punya active branch maupun inactive-container branch yang relevan.
- Menambahkan test untuk memastikan container-only branch tidak muncul lagi.

## Impacted Files

- `.docs/history/202603261430-prune-empty-container-branches-from-collection-preview.md` (added)
- `src-tauri/src/services/collection_preview_tree.rs` (modified)
- `src-tauri/src/services/tests/collection_preview_tree_tests.rs` (modified)

## Goal

Preview collection hanya menampilkan container yang benar-benar berisi mod terminal yang relevan.

## Impact

- Object dengan `0 mods` dan child container kosong tidak lagi muncul di preview.
- Branch inactive-container yang memang masih punya terminal child tetap dipertahankan.
