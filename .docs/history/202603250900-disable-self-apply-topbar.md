# Disable Self-Apply In Topbar

## Context

Dropdown collections di topbar masih membuka dialog apply saat user mengklik collection yang sedang aktif, padahal state itu tidak perlu diaplikasikan ulang.

## Changes

- Tambahkan guard di handler topbar agar collection aktif tidak bisa membuka `ApplyCollectionModal`.
- Tombol item collection aktif di dropdown sekarang di-disable dan diberi cursor non-interaktif.

## Impacted Files

- `src/components/layout/top-bar/ContextControls.tsx` (modified)

## Goal

Klik ulang collection yang sudah aktif dari topbar tidak lagi memicu apply dialog.

## Impact

- Flow apply dari topbar hanya berlaku untuk collection lain di corridor yang sama.
- Tidak ada perubahan schema, API, atau modal apply.
