# Update TRD for i18n CLI policy

## Context

TRD belum mencerminkan arsitektur i18n terbaru, aturan zero-hardcode localization, dan penggunaan `i18next-cli` untuk audit/sync locale FE.

## Changes

- Menambahkan dependensi frontend localization pada TRD.
- Menambahkan prinsip arsitektur untuk namespace locale, source locale `en`, dan larangan hardcoded user-facing strings.
- Memperbarui struktur proyek dengan direktori `src/locales/` dan `i18next.config.ts`.
- Menambahkan bagian QA untuk `i18n:lint`, `i18n:sync`, dan `i18n:status`.

## Impacted Files

- `.docs/trd.md` (modified)

## Goal

Dokumen teknis sekarang selaras dengan implementasi i18n FE dan tooling audit locale di repo.

## Impact

- Ekspektasi engineering untuk localization jadi eksplisit di level TRD.
- QA lokalization memiliki langkah verifikasi yang terdokumentasi.
- Tidak ada breaking change runtime.
