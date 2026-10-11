# Audit klasifikasi normal sync dan lock publikasi KeyViewer

Tanggal: 2026-10-10. Status: **disetujui pengguna; implementasi dan validasi lokal
selesai**. Hasil audit awal di bawah tetap merupakan pengukuran prototipe;
hasil produksi, mitigasi/fallback dan batas benchmark tercatat dalam
[execution report](normal-sync-keyviewer-execution-20261010.md).

## Keputusan implementasi dan mitigasi

- Normal sync menggunakan spare child budget delapan dengan collector dan pool
  yang ada. Scheduling root, strict terminal boundaries, urutan, dan propagasi
  error dipertahankan.
- Watcher saat ini mencakup Mods secara recursive dan parent konfigurasi runtime
  secara nonrecursive (`watcher/mod.rs`), sehingga belum membuktikan perubahan
  seluruh secondary include roots. Discovery KeyViewer tetap dilakukan di bawah
  revision/activation guard. Tidak menambah service coverage atau pre-scan yang
  berpotensi melewatkan duplikat baru.
- Sesuai fallback plan, alternatif yang diuji adalah pruning DISABLED hanya pada
  direktori menggunakan nama lokal, setelah satu pemeriksaan ancestor root.
  Output harus identik. Keputusan akhir menerima tradeoff terukur: guarded scan
  mayoritas DISABLED sekitar 79% lebih singkat, sedangkan fixture aktif memiliki
  median sekitar 1,5% lebih lama yang mungkin nyata. Tidak mengklaim nol overhead;
  detail final dan rentang lima putaran ada dalam execution report.
- Native E2E memakai private build/owned fixture melalui harness yang ada untuk
  full/scoped sync, primary/secondary duplicate migration, dan process restart.

## Keputusan

1. Prioritaskan pembagian klasifikasi immediate children untuk sedikit root besar.
   Prototipe mengurangi waktu klasifikasi sekitar 51% pada full sync dan 49% pada
   scoped sync, dengan hasil identik. Tidak ada manfaat terukur pada kontrol banyak
   root.
2. Kurangi traversal KeyViewer di critical section hanya setelah kelengkapan
   discovery dan perubahan filesystem selama persiapan dapat dibuktikan aman.
   Traversal menahan dua lock sekitar 250 ms pada fixture besar, termasuk ketika
   tidak ada duplikat. Memindahkan scan begitu saja belum aman.
3. Jangan menerapkan pruning DISABLED yang diuji secara umum: fixture seluruhnya
   aktif justru melambat sekitar 25%. Tidak perlu cache persisten, pool baru,
   scheduler baru, atau framework traversal umum.

## Metode dan batas bukti

Semua pengukuran memakai fixture sintetis milik pengujian di OS TempDir, binary
debug Windows yang sama, dan lima putaran berurutan dengan urutan pembanding
bergantian. Setup fixture dan assertion parity berada di luar timer. Cache OS
tidak dikontrol; hasil ini bukan cold-disk, release build, total startup, atau
latensi UI. Tidak membuka aplikasi maupun mengakses library GIMI asli.

Binary benchmark disalin sebelum pengukuran agar build chat lain tidak mengganti
artifact yang sedang dipakai. SHA-256:
`0da5263bd78ad1bbfe67bffbb80fa05d81cd22a3d45c8bb12b6fe510db3af928`.
Pool Rayon teramati 24 thread; angka ini bergantung environment, bukan konstanta
produksi. Hanya helper/prototipe `cfg(test)` yang ditambahkan pada dua file test.

## 1. Normal sync pada satu root besar

### Temuan

`disk_snapshot.rs` menjalankan root terpilih melalui `par_iter()`, sedangkan
immediate children dan rekursi klasifikasi dalam satu root berjalan serial.
Collector menjaga batas terminal mod, identitas filesystem, dan klasifikasi
strict. Jalur ini dipakai full sync, scoped sync, dan inspeksi recovery; onboarding
dengan snapshot yang telah disiapkan menggunakan jalur berbeda.

Produksi **tidak memiliki batas global delapan root**. Prototipe mempertahankan
scheduling root yang ada dan membagi spare budget delapan ke child branches ketika
root terpilih sedikit. Rekursi di bawah child tetap serial.

| Fixture / jalur                    | Baseline median | Prototipe median | Hasil                |
| ---------------------------------- | --------------: | ---------------: | -------------------- |
| 1 root, 2.000 mod campuran, full   |      503,378 ms |       244,171 ms | 51,49% lebih singkat |
| 1 root, 2.000 mod campuran, scoped |      487,106 ms |       246,437 ms | 49,41% lebih singkat |
| 32 root kecil, 64 mod, full        |        7,338 ms |         7,290 ms | Selisih noise        |
| 16 root seimbang, 2.048 mod, full  |      198,911 ms |       200,510 ms | Selisih noise        |

Rentang full satu root: baseline 484,867–521,540 ms; prototipe
236,496–256,325 ms. Rentang scoped: 474,078–510,660 ms dibanding
240,683–260,083 ms. Seluruh trial memeriksa parity lengkap field projection,
urutan, identitas fisik, dan count klasifikasi. Budget dua tidak memberi manfaat;
empat memberi perbaikan lebih kecil daripada delapan pada fixture ini.

Discovery produksi satu root, termasuk census, memiliki median 772,663 ms.
Median proporsi klasifikasi per trial sekitar 72,10%. Pengukuran ini menunjukkan
klasifikasi relevan terhadap discovery, tetapi **belum mengukur optimized discovery
atau total sync aplikasi**. Ada 100 trial klasifikasi dan 15 trial discovery.
Inventory proses sebelum, di tengah, dan sesudah run tidak menunjukkan compiler
atau benchmark pesaing.

### Plan minimal

1. Buat helper lokal pengumpulan immediate children di `disk_snapshot.rs` yang
   memanggil collector strict yang sudah ada. Jangan menggabungkan collector
   normal dengan collector onboarding yang memiliki kontrak berbeda.
2. Pertahankan `par_iter()` root. Bagi spare child budget delapan berdasarkan
   jumlah root terpilih; gunakan jalur serial saat budget satu. Jangan menambah
   parallel recursion atau membatasi fanout root lama. Gabungkan hasil child
   secara terurut dan pertahankan propagasi error.
3. Pertahankan authority/cancellation dan progress yang ada. Counter direktori
   adalah direktori yang dikunjungi/dicoba, bukan jumlah mod selesai. Jangan
   menjadikannya persentase mod tanpa denominator yang benar. Perubahan kontrak
   progress, bila diperlukan, harus menjadi perubahan terpisah yang kecil.
4. Validasi scoped/full pada 1, 3, dan 9 root serta jumlah child yang tidak habis
   dibagi budget. Cover terminal/variant/asset, nested container, hidden,
   DISABLED, Unicode, urutan/identity, error dan superseded activation. Hasil
   parsial tidak boleh diterbitkan sebagai sync sukses.

Target penerimaan: rentang benchmark satu root terpisah dari baseline dengan
parity identik; kontrol banyak root tidak regresi secara konsisten. Ukur full
discovery dan native full/scoped sync pada private build setelah implementasi.
Fixture dengan seluruh pekerjaan di satu child nested masih serial; manfaat
untuk bentuk tersebut belum terbukti. Rollback cukup mengembalikan scheduling
child ke serial tanpa migrasi state/data.

## 2. Traversal KeyViewer selama lock publikasi

### Temuan

Pada publikasi generation yang memerlukan migrasi duplikat,
`migrate_duplicate_keyviewer_entrypoints()` menelusuri effective include roots di
dalam `commit_if_current_sync_revision()`. Closure tersebut memegang mutex revisi
global dan mutex activation authority. Nama file dan ancestor DISABLED baru
difilter setelah WalkDir turun ke subtree.

Shortcut artifact yang tidak berubah melewati publikasi ini. Scan ketika
KeyViewer dinonaktifkan berada di luar guarded commit. Jadi temuan ini tidak
berarti seluruh startup atau seluruh request KeyViewer memegang lock sepanjang
scan.

| Fixture                                                 | Median guarded scan | Revision waiter | Activation waiter |
| ------------------------------------------------------- | ------------------: | --------------: | ----------------: |
| 32 mod aktif, 331 entry, 2 duplikat                     |            4,868 ms |        4,777 ms |          4,756 ms |
| 2.048 mod aktif, 20.504 entry, 0 duplikat               |          256,801 ms |      256,739 ms |        256,726 ms |
| 2.048 mod, mayoritas DISABLED, 20.522 entry, 2 duplikat |          247,626 ms |      247,467 ms |        247,462 ms |

Benchmark memanggil traversal aktual di bawah kedua guard aktual dan mengukur
dua waiter yang disinkronkan. Semua run mengonfirmasi mutex revisi masih held.
Rentang fixture aktif besar 234,357–289,758 ms; fixture mayoritas DISABLED
223,101–260,191 ms. Biaya pada library kecil relatif rendah.

Simulasi persiapan di luar lock memungkinkan kedua waiter selesai lebih dahulu,
kemudian authority lama ditolak dalam 9–13 mikrodetik. **Itu waktu penolakan stale
work, bukan waktu publikasi yang berhasil.** Discovery tetap memerlukan traversal;
journal, backup, rename, pointer publication, serta IPC/UI tidak diukur.

Run pertama dengan compiler overlap hanya eksploratori. Run terkontrol selesai
UTC 12:29:23; compiler berikutnya mulai 12:30:43. Cargo lain mulai pada tiga detik
terakhir tanpa rustc teramati saat pengukuran. Hasil tidak mengasumsikan seluruh
mesin idle.

### Plan dengan gate keselamatan

1. Pisahkan discovery dari mutasi melalui prepared candidates lokal per invocation.
   Discovery harus membedakan hasil lengkap kosong dari error traversal/read;
   helper lama yang melewatkan error tidak cukup sebagai bukti aman.
2. Sebelum memindahkan discovery keluar lock, buktikan coverage/invalidation semua
   effective include roots. Watcher configured Mods saja belum membuktikan
   secondary include roots. Validasi kandidat lama tidak mendeteksi duplikat baru
   yang muncul sesudah pre-scan.
3. Di commit, tetap gunakan revision dan activation guard serta serialisasi
   publikasi per game. Revalidasi konfigurasi include, containment canonical,
   identitas fisik, verified content/ownership, dan ketersediaan backup. Preserve
   journal recovery, rollback, serta rename tanpa menimpa tujuan yang muncul
   bersamaan. Pakai primitives filesystem yang ada setelah kontraknya diverifikasi.
4. Perubahan coverage/config/identity atau discovery tidak lengkap harus membatalkan
   prepared work dan meminta recovery/retry yang terikat authority; jangan
   menerbitkan success. Untuk root yang tidak memiliki bukti coverage, pertahankan
   scan dalam commit. Jika coverage memerlukan service/state baru yang besar,
   tunda pemindahan lock dan cari pengurangan traversal yang lebih kecil dahulu.
5. Uji perubahan file kandidat, duplikat baru termasuk di secondary roots,
   pergantian include root/junction, backup muncul, revision/activation superseded,
   serta kegagalan journal/rollback. Ukur publikasi sukses dan lock wait setelah
   perubahan; ukur native switching pada private build terisolasi.

Target: waktu lock tidak bertumbuh dengan seluruh tree pada root yang memiliki
bukti coverage, sambil mempertahankan ownership, completeness, dan recovery.
Untuk root tanpa coverage, fallback boleh tetap mahal. Tidak mengganti mutex
menjadi async atau menghapus guard; mengurangi pekerjaan di critical section
merupakan sasaran. Mutex memblokir thread sampai guard dilepas, dan critical
section pendek mengurangi contention.
[Rust Mutex](https://doc.rust-lang.org/stable/std/sync/struct.Mutex.html),
[Tokio shared state](https://tokio.rs/tokio/tutorial/shared-state).

### Kandidat yang tidak diteruskan

Pruning percobaan memeriksa ancestor DISABLED di setiap entry. Pada fixture
mayoritas DISABLED, median turun 232,687 → 65,445 ms, tetapi pada fixture aktif
naik 252,437 → 316,584 ms, sekitar 25,4%. Jangan mengadopsinya secara umum.
Pruning dengan predicate hanya pada direktori mungkin lebih murah, tetapi belum
diukur dan bukan bagian implementasi yang disetujui.

## Artifact dan validasi audit awal

Semua artifact lokal ada di `.audit-output/normal-sync-keyviewer-20261010/`:

- `benchmark-build.json`: provenance binary bersama.
- `normal-sync-children.json` dan `.log`: hasil klasifikasi/discovery dan parity.
- `keyviewer-lock-controlled.json` dan `.log`: hasil lock/waiter/pruning final.
- `keyviewer-lock-exploratory.json` dan `keyviewer-lock.log`: run awal dengan
  compiler overlap, tidak dipakai sebagai hasil final.

Kedua ignored benchmark lulus pada binary snapshot yang sama. Scoped rustfmt,
`git diff --check`, dan `cargo clippy --lib --tests -- -D warnings` lulus.
Tidak ada perubahan perilaku produksi
atau validasi native baru pada sesi audit awal. Implementasi yang disetujui kini
selesai; source produksi dan native final dicatat dalam execution report.
