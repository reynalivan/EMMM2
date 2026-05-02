# Collection Preview Terminal Types and Inactive Section

## Context

Collection preview masih menampilkan subfolder variant/modpack yang seharusnya collapse ke root utama, belum menyimpan metadata tipe mod di snapshot collection, dan branch yang enabled tapi tidak berdampak karena parent container disabled masih tercampur di tree utama.

## Changes

- Menambahkan metadata snapshot mod di `collection_mods`: `preview_path`, `node_type`, dan `warnings_json`.
- Saat create/update unsaved collection, backend sekarang menyimpan terminal preview target per mod:
  - `FlatModRoot` tetap leaf terminal
  - `ModPackRoot` collapse ke folder utama
  - `VariantContainer` collapse ke folder container utama
- Builder preview tree sekarang:
  - memakai metadata snapshot tersimpan bila ada
  - fallback classify on-the-fly untuk live corridor preview
  - menggabungkan duplicate variant/modpack root ke satu node
  - memindahkan branch yang kena disabled container ke section bawah `Inactive Containers`
  - memberi status `Disabled` pada container parent dan `Disabled by Container` pada child yang tidak berdampak
  - membawa warning/corrupt ke node preview
- Treeview collections dipoles agar:
  - chip type muted muncul untuk `Container`, `Variants`, `Mod Pack`, `Flat Mod`
  - warning icon tampil untuk node corrupt
  - inactive container branch tampil di panel terpisah paling bawah

## Impacted Files

- `.docs/history/202603251515-collection-preview-terminal-types-and-inactive-section.md` (added)
- `src-tauri/migrations/20260325150000_collection_mod_preview_metadata.sql` (added)
- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/services/collection_preview_tree.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/pipeline/steps/snapshot_state.rs` (modified)
- `src-tauri/src/services/tests/collection_preview_tree_tests.rs` (modified)
- `src/features/collections/components/CollectionTreeView.tsx` (modified)
- `src/features/collections/components/CollectionTreeView.test.tsx` (modified)
- `src/features/collections/utils/buildModTree.test.ts` (modified)
- `src/types/collection.ts` (modified)
- `src/locales/en/collections.json` (modified)
- `src/locales/id/collections.json` (modified)
- `src/locales/zh/collections.json` (modified)

## Goal

Preview collection sekarang menampilkan terminal node yang benar untuk flat mod, mod pack, dan variant container, sambil memisahkan branch yang tidak berdampak karena disabled container agar tree utama tetap akurat dan count tetap bersih.

## Impact

- Count preview hanya menghitung branch aktif yang benar-benar berdampak.
- Saved collections sekarang menyimpan metadata preview mod agar render variant/modpack lebih stabil.
- Ada migration DB baru untuk metadata `collection_mods`.
- Runtime Rust test masih belum bisa dieksekusi penuh di environment ini karena `STATUS_ENTRYPOINT_NOT_FOUND`.

## Notes

- Branch inactive dipisah di preview tree, bukan disembunyikan, supaya user tetap bisa audit jalur folder yang sedang tidak berdampak.
