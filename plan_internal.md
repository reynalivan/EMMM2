## Phase H (Part 2 & 3) - Struktur Internal Modul

### Tantangan Utama (`api.rs` & *Visibility*)
Panduan Anda menyarankan agar `mod.rs` tidak mempublikasikan (`pub mod`) isi internal:
```rust
// Jangan:
pub mod domain;
pub mod adapters;
pub mod application;
```
Di Rust, *integration tests* (folder `tests/`) dan file modul lain saat ini memanggil langsung ke struktur internal (`crate::modules::X::application::Y`). Jika kita menutup aksesnya seketika, kompilator Rust akan memuntahkan ratusan error *Private Module* secara instan.

Mengekstrak semuanya ke `api.rs` secara manual sangat berisiko merusak fungsionalitas jika dilakukan secara "membuta" dalam satu langkah.

### Proposed Plan

**Step 1: Merapikan Layer Adapters (Aman & Cepat)**
Membuang terminologi `inbound`/`outbound` yang membuat hierarki terlalu dalam, dan menggantinya dengan penamaan teknologi murni.
- `adapters/inbound/*` ➔ `adapters/tauri/*`
- `adapters/outbound/sqlite/*` ➔ `adapters/sqlite/*`
- `adapters/outbound/fs/*` ➔ `adapters/fs/*`
- Semua *import paths* akan disesuaikan.

**Step 2: Menyiapkan Pondasi `api.rs` (Transisi)**
- Membuat file `api.rs` di dalam **setiap modul**.
- Memindahkan semua ekspor dari `facade.rs` (jika ada) ke `api.rs`.
- Mengubah `pub mod application;` menjadi `pub(crate) mod application;` secara bertahap, dan mengarahkan *Tauri commands* di `lib.rs` untuk mulai membaca dari `crate::modules::X::api::*`.

---
Saya akan mengeksekusi **Step 1 (Layer Adapters)** terlebih dahulu menggunakan Python *script* massal, karena ini akan membuat folder modul kita jauh lebih pipih (*flat*) dan elegan. Setelah berhasil dan `cargo test` hijau, kita baru masuk ke kerumitan *Step 2*.
