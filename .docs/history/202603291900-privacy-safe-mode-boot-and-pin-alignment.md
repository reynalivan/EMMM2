# Privacy Safe Mode Boot And Pin Alignment

## Context

`req-30` masih punya gap nyata di boot PIN guard dan lockout policy. App mengunci UI di kondisi yang salah, dan lockout PIN masih memakai model persisted 15 menit alih-alih 60 detik memory-backed.

## Changes

- Boot guard di startup sekarang memakai `check_boot_security(...)` backend, bukan heuristik frontend `safeMode || hasPin`.
- `pin_service` sekarang memakai guard in-memory 60 detik untuk failed PIN attempts.
- Status PIN frontend sekarang dibaca dari memory guard, bukan lockout DB lama.
- Set/Clear PIN mereset guard memory dan membersihkan compatibility counter DB.
- Requirement doc `req-30` dirapikan supaya executive summary konsisten dengan model dual-corridor yang aktif.
- Regression tests ditambah untuk startup lock screen dan PIN modal flow.
- Rust tests ditambah untuk 60s lockout dan unsafe-only boot lock contract.

## Impacted Files

- `.docs/requirements/req-30-privacy-safe-mode.md` (modified)
- `src/App.tsx` (modified)
- `src/App.test.tsx` (modified)
- `src/lib/bindings.ts` (modified)
- `src/features/safe-mode/PinEntryModal.test.tsx` (modified)
- `src-tauri/src/services/pin_service.rs` (modified)

## Goal

Boot privacy lock sekarang hanya aktif saat startup di corridor Unsafe dengan PIN, dan brute-force PIN mengikuti kontrak 5 attempts + 60s memory-backed lockout.

## Impact

- Startup routing Safe corridor tidak lagi terkunci salah.
- Lockout PIN tidak lagi survive lewat DB restart state lama.
- Command contract frontend/backend untuk boot security menjadi satu jalur.
- Tidak ada migration DB baru.

## Notes

- Runner Rust Windows lokal masih gagal mengeksekusi binary test tertentu dengan `STATUS_ENTRYPOINT_NOT_FOUND`, jadi verifikasi Rust untuk batch ini memakai compile-success dan unit-test source audit.
