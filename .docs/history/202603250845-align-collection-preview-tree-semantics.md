# Align Collection Preview Tree Semantics

## Context

Collection preview sebelumnya membangun tree dari path mentah sehingga panel kanan tidak memahami klasifikasi folder Epic 11, tidak menandai inherited inactive, dan tetap membuka child `VariantContainer`.

## Changes

- Preview API collections/apply/corridor sekarang mengembalikan `tree_nodes` siap-render dari backend.
- Ditambahkan builder shared untuk merekonstruksi tree berdasarkan `CollectionObject.path_key`, klasifikasi folder, disabled ancestor, dan flatten `VariantContainer`.
- `ContainerFolder` multi-level sekarang tetap muncul penuh di tree.
- Parent `ContainerFolder` yang membuat branch tidak efektif sekarang diberi state `Inactive` + warning reason.
- `VariantContainer` berhenti di node container utama tanpa merender child aktif di bawahnya.
- Terminal rows yang tidak efektif tetap terlihat sebagai node dimmed.
- Snapshot object path pada create/dirty/live preview diselaraskan ke `folder_path`, bukan object id.
- `CollectionTreeView` dipindah ke payload tree baru, dengan fallback legacy untuk consumer lama.

## Impacted Files

- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/domain/corridor.rs` (modified)
- `src-tauri/src/services/mod.rs` (modified)
- `src-tauri/src/services/collection_preview_tree.rs` (added)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/tests/collection_preview_tree_tests.rs` (added)
- `src/types/collection.ts` (modified)
- `src/features/collections/components/CollectionTreeView.tsx` (modified)
- `src/features/collections/components/CollectionTreeView.test.tsx` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src/features/collections/utils/buildModTree.test.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)

## Goal

Preview collections sekarang menunjukkan hierarki object/container/mod yang sesuai dengan semantics folder listing, termasuk awareness untuk branch yang secara inherited tidak akan aktif.

## Impact

- Sidebar preview collections, apply preview, dan safe-mode switch preview memakai tree semantics yang sama.
- Tidak ada migration schema DB.
- Payload preview frontend berubah dengan penambahan `tree_nodes`, `current_tree_nodes`, `target_tree_nodes`, `leaving_tree_nodes`, dan `target_tree_nodes`.
- Runtime unit test Rust masih belum bisa dieksekusi di environment ini karena binary test crash `STATUS_ENTRYPOINT_NOT_FOUND`, tetapi compile test berhasil.

## Notes

- `CollectionTreeView` masih menerima `members` sebagai fallback untuk rollout bertahap.
- `VariantContainer` sengaja ditampilkan sebagai terminal row walau snapshot path sebenarnya masuk ke child variant tertentu.
