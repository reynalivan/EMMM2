# Remove Container Chip From Collection Tree

## Context

Generic `ContainerFolder` chip membuat tree preview terasa seperti semua node adalah container dan menutupi metadata `VariantContainer` yang sebelumnya sudah lebih jelas.

## Changes

- Hapus chip `Container` dari collection preview tree.
- Kembalikan icon folder generic untuk `ContainerFolder`.
- Pertahankan chip muted untuk `Variants`, `Mod Pack`, dan `Flat Mod`.
- `Inactive` tetap sebagai chip warning terpisah.

## Impacted Files

- `src/features/collections/components/CollectionTreeView.tsx` (modified)
- `src/features/collections/components/CollectionTreeView.test.tsx` (modified)

## Goal

Tree preview kembali menonjolkan node type yang benar-benar penting tanpa membuat semua folder tampak seperti metadata container.
