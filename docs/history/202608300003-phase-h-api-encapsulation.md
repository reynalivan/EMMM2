# Refactor: Encapsulate Modules with api.rs (Phase H - Part 3)

## Context
Mengunci seluruh arsitektur modul agar mengarah ke prinsip *Modular Monolith* yang murni. Menutup akses publik ke `domain`, `application`, dan `adapters`, sehingga komunikasi lintas-modul atau dari pihak eksternal hanya bisa terjadi melalui `api.rs`.

## Changes
- Menambahkan `api.rs` di setiap 16 modul.
- Menurunkan *visibility* `mod domain`, `mod application`, dan `mod adapters` di seluruh `mod.rs` menjadi `pub(crate)`.
- Mengimplementasikan *Testing Backdoor* elegan di dalam `api.rs`:
  ```rust
  #[cfg(debug_assertions)]
  pub mod testing {
      /* selective re-exports */
  }
  ```
  Ini mengizinkan *integration tests* untuk tetap menyuntik data langsung ke SQLite adapters selama masa testing, tanpa mengotori API publik kita.
- Meng-update ratusan rujukan di `tests/*.rs` untuk memanggil via `api::testing::*`.

## Impacted Files
- `src-tauri/src/modules/*/mod.rs` (visibilitas diubah)
- `src-tauri/src/modules/*/api.rs` (dibuat/ditulis ulang)
- `src-tauri/tests/*.rs` (rute disesuaikan)

## Goal
Tercapainya *Information Hiding* penuh di EMMM2. Tiap modul kini adalah kotak hitam sejati. 

## Impact
- `cargo test` lulus 100% (865 passed).
- Modul internal tidak akan terekspos lagi oleh IDE intellisense secara serampangan kepada file di luar modulnya.
