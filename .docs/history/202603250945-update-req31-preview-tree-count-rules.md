# Update Req-31 Preview Tree And Count Rules

## Context

Requirement collections belum mendokumentasikan semantics tree preview terbaru dan aturan count yang sudah dipakai UI/backend.

## Changes

- Tambahkan acceptance criteria untuk preview treeview collections.
- Dokumentasikan flatten `VariantContainer`, full `ContainerFolder` chain, inherited inactive marker, dan visibility untuk disabled terminal rows.
- Dokumentasikan aturan count baru: leaf mod = 1, visible `VariantContainer` = 1, parent container = 0.
- Tambahkan integration point bahwa list/topbar count harus memakai preview-tree semantics, bukan raw snapshot rows.
- Tambahkan note bahwa active collection di topbar tidak boleh self-apply.

## Impacted Files

- `.docs/requirements/req-31-collections.md` (modified)

## Goal

Dokumen Epic 31 sekarang selaras dengan implementasi preview tree dan mod count yang sedang berlaku.
