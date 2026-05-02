# V2 corridor strict runtime and current preview recovery

## Context

Refactor v2 memindahkan surface FE/BE ke `get_corridor_state` dan preview v2, tetapi semantik active collection, apply/save parity, manual drift, dan preview current runtime ikut regress.

## Changes

- `get_corridor_state` di-backend kembali dihitung dari runtime disk/signature strict, lalu memilih named/unsaved match dengan pointer corridor hanya sebagai tie-break exact match.
- Save collection dibedakan menjadi dua jalur:
  - save current runtime: tetap membuat named preset aktif dari state live
  - save stored snapshot: clone snapshot tersimpan ke named preset tanpa mengubah active truth
- Collections page sekarang membedakan source preview `current_runtime` vs `stored_collection`.
- Row sintetik current runtime muncul hanya saat current corridor strict state benar-benar unsaved.
- Preview panel kanan bisa merender strict current runtime langsung dari `CorridorSnapshot`, bukan hanya preview koleksi tersimpan.
- Mutasi mod/folder yang mengubah corridor state sekarang memaksa refetch strict `get_corridor_state`, bukan hanya invalidate generik.
- Active game bootstrap sekarang prefetch corridor snapshot dan list collections sekaligus.

## Impacted Files

- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src/lib/bindings.ts` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/collections/types.ts` (added)
- `src/features/collections/CollectionsPage.tsx` (modified)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/components/CollectionList.test.tsx` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/hooks/useFolderCoreMutations.ts` (modified)

## Goal

Arsitektur v2 tetap dipakai, tetapi current/topbar/apply/switch kembali membaca strict runtime truth yang sama dan Collections page kembali bisa membedakan state live vs snapshot tersimpan.

## Impact

- Active named collection sekarang bisa kembali stabil setelah save/apply bila runtime memang match.
- Manual drift mod/folder lebih cepat menjatuhkan active state ke `Unsaved Preset`.
- Collections page tidak lagi mencampur current runtime dengan stored unsaved snapshot.
- Ada penambahan refetch eksplisit setelah mutasi corridor-affecting, jadi update state lebih deterministik dengan sedikit biaya query tambahan.

## Notes

- Jalur save snapshot as named ditambahkan dengan memperluas `create_collection`, bukan menghidupkan command lama.
- Verifikasi backend runtime dilakukan lewat compile penuh Rust; beberapa unit test Rust tidak bisa dijalankan di environment ini karena executable test process gagal start di Windows.
