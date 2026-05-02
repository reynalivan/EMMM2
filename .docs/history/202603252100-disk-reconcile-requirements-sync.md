# Disk Reconcile Requirement Sync

## Context

Requirement docs masih menyebut watcher lama, key cache lama, dan alur refresh pra-Disk-Reconcile, sehingga rawan membuat implementasi berikutnya kembali ke pola obsolete.

## Changes

- Menyelaraskan requirement watcher agar watcher menjadi trigger-only dan runtime truth tetap lewat Disk Reconcile.
- Menambahkan `InternalMutation` dan runtime side effects ke requirement INI viewer dan image gallery.
- Memperbarui requirement collections agar dirty-state berasal dari hasil Disk Reconcile, bukan asumsi watcher lama.
- Memperbarui requirement game switch, workspace shell, folder listing, dan preview panel agar memakai istilah/query key runtime yang benar.
- Mengganti referensi command thumbnail lama ke command yang sekarang dipakai frontend.

## Impacted Files

- `.docs/requirements/req-02-game-management.md` (modified)
- `.docs/requirements/req-05-workspace-layout.md` (modified)
- `.docs/requirements/req-11-folder-listing.md` (modified)
- `.docs/requirements/req-16-preview-panel-layout.md` (modified)
- `.docs/requirements/req-18-ini-viewer.md` (modified)
- `.docs/requirements/req-19-image-gallery.md` (modified)
- `.docs/requirements/req-28-file-watcher.md` (modified)
- `.docs/requirements/req-31-collections.md` (modified)

## Goal

Requirement docs sekarang membaca Disk Reconcile dan Deep Match Scanner sebagai dua domain terpisah, dengan runtime refresh, cache invalidation, dan internal file-write flow yang konsisten terhadap implementasi saat ini.

## Impact

- Mengurangi risiko agent atau implementasi berikutnya menghidupkan lagi watcher invalidate-only flow.
- Mengurangi drift antara requirement, flow architecture, dan kode runtime.
- Tidak ada breaking runtime change; ini sinkronisasi dokumentasi.
