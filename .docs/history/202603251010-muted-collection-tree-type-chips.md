# Muted Collection Tree Type Chips

## Context

Chip `VariantContainer` di collection preview tree masih memakai warna accent, dan node type lain belum konsisten menampilkan chip metadata.

## Changes

- Tambahkan renderer chip type shared di treeview preview collections.
- `VariantContainer`, `ContainerFolder`, `ModPackRoot`, dan `FlatModRoot` sekarang menampilkan chip metadata.
- Semua chip type dibuat muted dan theme-aware dengan class neutral/base.
- `VariantContainer` icon juga dibuat muted, tidak lagi memakai highlight accent.
- `Inactive` tetap dipertahankan sebagai chip warning terpisah.

## Impacted Files

- `src/features/collections/components/CollectionTreeView.tsx` (modified)
- `src/features/collections/components/CollectionTreeView.test.tsx` (modified)

## Goal

Metadata type di tree preview sekarang lebih konsisten dan tidak mengganggu hierarchy dengan warna accent yang terlalu dominan.

## Impact

- Tidak ada perubahan payload backend atau logic count.
- `tsc --noEmit` lolos.
- Vitest runtime di environment ini masih terblokir `esbuild spawn EPERM`.
