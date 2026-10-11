# Enable global workspace hotkeys

## Context

Pengguna meminta Safe Mode dan preset tidak dibatasi game foreground, lalu
memilih agar F7 tetap hanya saat game fokus tanpa memindahkan fokus otomatis.

## Changes

- Dispatch Safe Mode/next/previous preset tidak lagi memerlukan game fokus.
- Validasi ulang konteks tidak memeriksa fokus; hotkey enabled, game pilihan,
  path dan identitas native Mods root tetap diperiksa setelah menunggu.
- F7 serta synthetic reload tetap memeriksa fokus sebelum pengiriman input.
- Regression memakai temporary Mods root dan game tanpa executable, tanpa
  mengirim input OS atau menyentuh library asli.

## Impacted Files

- `src-tauri/src/modules/automation/application/hotkeys/manager.rs` (modified).
- `src-tauri/src/modules/automation/application/hotkeys/mod.rs` (modified).
- `src-tauri/src/modules/automation/application/hotkeys/tests/manager_tests.rs` (modified).
- `src-tauri/AGENT.md` (modified).
- This history file (added).

## Goal and Impact

Workspace hotkeys tetap berfungsi ketika aplikasi lain aktif atau game tidak
berjalan. Tidak ada perubahan dependency, schema, shortcut default, cooldown,
mutation admission, registration atau focus transfer. Game berjalan di
background dapat tetap memerlukan reload manual setelah perubahan tersimpan.

## Validation

Regression RED terkonfirmasi sebelum patch: assertion `context.validate` gagal
(0 passed/1 failed; coordinated Rust runner, RTK tee `1791644426_cargo_test.log`).
GREEN compiled testlib lulus: 1.765 passed/32 ignored, termasuk regression positif
dan guard game pilihan/hotkey disabled/root replacement (coordinated runner).
Scoped rustfmt, whitespace dan strict production Clippy
(`cargo clippy --lib -- -D warnings`) lulus. Ketiga source/test hotkey cocok
dengan shared compiled source snapshot
`.audit-output/runtime-indexing-implementation-20261010/benchmark-source.json`.
All-targets Clippy tertahan test browser baru yang belum selesai di task lain;
pengiriman shortcut OS lintas aplikasi belum diverifikasi secara native.
Aplikasi terpasang tidak diganti atau direstart.
