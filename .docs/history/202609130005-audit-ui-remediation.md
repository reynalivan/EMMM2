# Audit UI remediation

## Context

Audit UI menemukan clipping mobile, QuickLiquid kosong, dialog yang tidak aksesibel, copy tidak konsisten, dan feedback Discover yang tidak terlihat.

## Changes

- Meneruskan tinggi QuickLiquid, menghapus penutup overflow global, dan menambahkan token copy sekunder serta hit target touch.
- Memindahkan enam modal ke native dialog dan mengganti konfirmasi Discover dengan dialog aplikasi.
- Melokalkan validasi Create Object dan menghapus em dash pada copy audit.

## Impacted Files

- `src/app/entrypoint/App.css` (modified)
- `src/pages/onboarding/components/ManualSetupForm.tsx` (modified)
- `src/pages/browser/components/BrowserPage.tsx` (modified)
- `src/shared/lib/hooks/useDialogSync.ts` (modified)
- modal, locale, dan fallback UI terkait (modified)

## Goal

Membuat surface audit lebih tahan terhadap mobile, keyboard, dan kegagalan aksi.

## Impact

Recovery mempertahankan explicit-action requirement; dialog tidak dapat ditutup dengan Escape karena itu tidak boleh mengabaikan recovery task.
