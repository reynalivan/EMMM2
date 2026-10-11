# Implementasi normal sync dan pruning KeyViewer

Tanggal: 2026-10-10. Plan disetujui pengguna. Implementasi lokal, targeted gates,
benchmark produksi, dan validasi native selesai.

## Perubahan dan mitigasi

- Normal full/scoped sync membagi spare budget delapan ke immediate children
  ketika root terpilih sedikit. Root `par_iter`, collector strict, urutan,
  physical identity, terminal boundaries, dan propagasi error dipertahankan.
  Budget satu memakai loop serial yang sama; rekursi lebih dalam tetap serial.
- Discovery duplikat KeyViewer tetap di dalam revision/activation guard karena
  watcher belum mencakup seluruh secondary include roots. Tidak memakai hasil
  pre-scan yang dapat melewatkan duplikat baru.
- Traversal memeriksa ancestor root sekali, lalu melewati subtree DISABLED
  berdasarkan prefix ASCII nama direktori sebelum descent. Filter ownership,
  deduplication, include roots, kebijakan junction, journal dan rollback tetap
  digunakan. Tidak ada pool, cache persisten, service, dependency atau schema baru.
- Review menemukan kesalahan spec native: ManualRepair selalu full. Spec scoped
  diperbaiki menjadi WatcherBatch dengan path fixture absolut, tetap meminta
  assertion Scoped.

## Hasil benchmark produksi

Binary debug privat dengan SHA-256
`93deca22ff12fc2e7a6afe226f0b393545c5b5c992ca08995f48f8137bd47645`
memakai fixture sintetis OS TempDir dan lima putaran bergantian. Setup/assertion
parity tidak masuk timer. Semua hasil projection diperiksa lengkap, termasuk
urutan, physical identity dan count klasifikasi. Cache OS tidak dikontrol.

### Normal sync

| Fixture / jalur                | Baseline serial reference | Produksi width 8 |
| ------------------------------ | ------------------------: | ---------------: |
| Satu root / 2.000 mod / full   |              1.229,509 ms |       323,455 ms |
| Satu root / 2.000 mod / scoped |              1.505,799 ms |       348,410 ms |
| 32 root kecil / 64 mod         |                 11,078 ms |        13,094 ms |
| 16 root seimbang / 2.048 mod   |                301,846 ms |       219,122 ms |

Terdapat 100 classification trial dan 15 discovery trial, seluruhnya exact
parity. Range full baseline 486,996–1.487,833 ms versus produksi
310,484–578,183 ms; scoped 1.311,803–1.713,441 ms versus
321,376–597,061 ms. Rentang full masih overlap pada ujungnya; median berubah
jelas dan kontrol produksi width satu tetap mendekati serial reference.

Kontrol banyak root memakai child budget satu sehingga scheduling tidak berubah.
Rentang banyak root kecil overlap (10,644–15,564 versus 11,228–17,598 ms), dan
rentang root seimbang juga overlap. Selisih kontrol bukan bukti keuntungan dari
scheduling baru. Production discovery satu root memiliki median 861,222 ms;
tidak dibandingkan dengan optimized/serial discovery berpasangan, sehingga tidak
diklaim sebagai persentase percepatan discovery atau total startup.

Inventory sebelum dan di tengah run hanya mengamati proses benchmark milik
pengujian; tidak mengklaim seluruh beban OS terkontrol. Angka antar sesi tidak
dibandingkan sebagai before/after aplikasi.

### KeyViewer

Kedua pembanding menjalankan traversal di bawah lock produksi yang sama. Semua
30 guarded trial per run mengonfirmasi mutex revision held dan commit diterima;
dua waiter mengukur revisi game lain dan pergantian activation. Tidak ada simulasi
stale rejection yang dianggap publikasi sukses.

| Fixture                                  | Run pertama: baseline → produksi | Run ulang: baseline → produksi |
| ---------------------------------------- | -------------------------------: | -----------------------------: |
| Kecil / 32 mod aktif                     |                 3,420 → 3,389 ms |               5,842 → 5,863 ms |
| Besar / 2.048 mod aktif / tanpa duplikat |             419,084 → 406,353 ms |           484,257 → 478,587 ms |
| Besar / mayoritas DISABLED               |              413,468 → 88,166 ms |           492,598 → 119,435 ms |

Run pertama waiter revisi/activation pada mayoritas DISABLED turun sekitar
413 ms → 88 ms; run ulang sekitar 492 ms → 117 ms. Range guarded run ulang
baseline 465,042–537,352 ms versus produksi 102,858–134,978 ms, jauh terpisah.
Gain fixture aktif masih noise. Timer scan tanpa guard pada fixture aktif besar
berubah 405,877 → 410,308 ms dan 482,491 → 532,375 ms; run ulang memiliki
variance tinggi dan tidak dipakai sebagai bukti percepatan fixture aktif.

**Kedua run KeyViewer beririsan dengan compiler dari chat lain**, meskipun dimulai
setelah inventory idle dan permintaan slot. Run pertama compiler mulai UTC
13:12:09 selama pengukuran 13:11:45–13:12:43; compiler juga terlihat di tengah
run ulang 13:14:38–13:15:23. Angka ini berlabel pengukuran dengan background
load, bukan benchmark mesin idle. Manfaat pada mayoritas DISABLED tetap jauh
melampaui variance pada kedua run; active-control guarded ranges overlap dan
tidak menunjukkan regresi konsisten yang dapat diatribusikan ke kandidat.

### Gate akhir setelah review

Review menilai gate fixture aktif pada dua run di atas masih inconclusive karena
background compiler. Dilakukan satu paired run terakhir pada binary yang sama,
UTC 13:26:41–13:27:18; inventory compiler/native sebelum dan sesudah kosong.
Tidak menganggap seluruh cache dan beban OS terkontrol.

| Fixture                    | Baseline guarded median | Produksi guarded median | Keputusan                 |
| -------------------------- | ----------------------: | ----------------------: | ------------------------- |
| Kecil / aktif              |                3,234 ms |                3,254 ms | Selisih kecil             |
| Besar / seluruhnya aktif   |              383,909 ms |              389,568 ms | Potensi overhead 1,47%    |
| Besar / mayoritas DISABLED |              381,678 ms |               80,133 ms | Sekitar 79% lebih singkat |

Range aktif baseline 372,010–403,686 ms versus kandidat 379,645–405,107 ms;
overlap bukan bukti tidak ada regresi. Empat dari lima guarded pasangan aktif
lebih lambat dan satu lebih cepat; overhead median sekitar 5,659 ms dapat nyata.
Scan timer tanpa guard juga naik 380,341 → 385,308 ms, sekitar 1,3%.

Range mayoritas DISABLED tetap jauh terpisah: baseline 376,006–433,773 ms versus
kandidat 75,483–82,632 ms. Median waiter revisi/activation turun sekitar
381,5 → 79,9 ms. Seluruh guarded trial tetap held-lock dan commit accepted,
dengan output identik.

Pruning dipertahankan sebagai **tradeoff eksplisit**: menghilangkan traversal
subtree DISABLED memberi manfaat besar dengan potensi biaya aktif sekitar 1,5%,
tanpa state/dependency baru. Acceptance wording plan diperbarui untuk keputusan
ini; tidak menyatakan nol overhead. Review akhir menerima tradeoff tersebut.
Jika nol overhead aktif menjadi syarat mutlak, rollback minimal adalah
mengembalikan traversal KeyViewer lama sambil mempertahankan normal child split.

Ini tidak membuktikan keseluruhan publication selesai dalam angka tabel:
journal, backup, rename, resource verification dan pointer publication tidak
ditimer di benchmark ini. Library seluruhnya aktif tetap memiliki traversal
tree di bawah lock. Tidak mengklaim telah menghapus seluruh contention.

## Gate verifikasi

- RED traversal terbukti dari full-suite log sebelum pruning; assertion
  before-descent gagal tepat pada subtree `dIsAbLeD-legacy`.
- GREEN: 27 disk snapshot tests dan 58 post-apply tests lulus; 9 benchmark manual
  terkait tetap ignored dalam run unit. Empat risk test normal dan tiga test
  pruning/ownership/junction termasuk di dalam angka tersebut.
- Strict `cargo clippy --all-targets -- -D warnings` lulus pada source kandidat.
- Typecheck terarah spec native, ESLint spec, scoped rustfmt dan diff check lulus.
- Dua benchmark produksi lulus; dua run KeyViewer awal beririsan compiler dan
  run ketiga menutup gate akhir tanpa compiler/native teramati pada inventory.
- Native final lulus **2/2 spec, 2 kasus, tiga real process restart** melalui
  harness build baru dengan private identifier `com.reynalivan.emmm.e2e`.

## Native final

Build debug privat: SHA-256
`2ab61e9bde60902ea2a79b04bc1440cd7b9e58a50fd2373c0846802440882cf2`.
Source hash empat file Rust terkait dan spec baru cocok dengan kandidat benchmark.
Frontend typecheck/bundle dan backend build lulus melalui harness sebelum launch.

- `normal-sync-keyviewer.e2e.ts`: full dan scoped sync satu root dengan 513 mod,
  stable object ID/count, publikasi artifact, backup primary/secondary duplicate,
  serta konten DISABLED dan foreign file tetap utuh. Session
  `f29921d5cce722461364ef9500d266f2`.
- `startup-reopen.e2e.ts`: tiga restart native dengan 301 mod; tiap proses satu
  watcher, satu StartupBoot baseline, tanpa full GameSwitched tambahan. Background
  keybinding, disabled ancestor filtering, dan staging cleanup lulus. Session
  `9e807fae6df4bf2ad803294673114da6`.
- Shell observation `performance.now()` sebesar 425/380/433 ms adalah umur halaman
  ketika dashboard diamati, bukan latensi dari process launch atau core-ready.
  Tidak dihitung sebagai percepatan total startup atau p95.
- Kedua owned native process trees berhenti; harness menghapus dua fixture
  terjadwal sesudah shutdown. Inventory akhir tidak menunjukkan driver, E2E app,
  atau benchmark milik pengujian masih berjalan.

Checkout dibagi dengan chat lain yang mengubah module berbeda. Laporan ini
mensertifikasikan scope performa dan gate yang tercatat, bukan seluruh source
terbaru atau full-suite chat lain yang masih diperbaiki.

## Artifact dan batas data

Artifact ada di `.audit-output/normal-sync-keyviewer-implementation-20261010/`:
`candidate-source.json`, `candidate-build.json`, `keyviewer-red.log`,
`targeted-validation.json`, `benchmark_normal_sync_child_classification.{log,json}`,
`keyviewer-exploratory.{log,json}`, `keyviewer-repeat-with-load.{log,json}`, dan
`benchmark_keyviewer_traversal_publication_lock.{log,json}`. `native-build.json`
dan `native-validation.json` mencatat build/session/status native akhir.

Seluruh fixture dimiliki pengujian. Library `E:\Data\XXMI Launcher\GIMI` tidak
diakses dan config aplikasi terpasang tidak diganti. Ukuran asset dalam GB bukan
variabel benchmark ini; pengujian mengukur jumlah/struktur direktori dan INI.
Tidak menyimpulkan latensi untuk ratusan GB dari fixture sintetis ini.
