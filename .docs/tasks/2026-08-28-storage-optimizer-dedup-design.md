# Storage Optimizer: Ownership-Aware Dedup Design

**Tanggal:** 2026-08-28
**Status:** Proposed — menunggu approval implementasi
**Scope:** Audit dan rancangan; belum ada perubahan kode produksi

## Tujuan

Storage Optimizer harus menemukan data yang benar-benar redundan tanpa menyebut subvariant 3DMigoto sebagai duplicate mod. Aksi Trash atau hardlink hanya boleh tersedia ketika identitas konten sudah dikonfirmasi dengan full BLAKE3 pada unit mod yang independen.

## Temuan riset

GIMI merger tidak menghasilkan beberapa mod independen. Generator membaca INI di child folder, lalu membuat satu `merged.ini` yang:

- mendefinisikan variable cycle seperti `$swapvar`;
- menggabungkan override dengan target `(hash, match_first_index)` yang sama;
- menjalankan `CommandList` dengan cabang `if/else if` per nilai variant;
- mengikat resource dari child folder yang berbeda;
- menonaktifkan INI sumber dengan prefix `DISABLED` agar override lama tidak ikut dimuat.

Dengan demikian, root merged beserta seluruh child resource-nya adalah satu program runtime dan satu logical mod unit. Child folder di dalamnya bukan kandidat duplicate folder. Perilaku ini terlihat pada [GIMI merge generator](https://github.com/SilentNightSound/GI-Model-Importer/blob/main/Tools/genshin_merge_mods.py) dan cocok dengan sampel library lokal `E:/Data/XXMI Launcher/GIMI/Mods/character/Albedo/DISABLED Albedo Flowery/merged.ini`.

3DMigoto mengeksekusi `TextureOverride`, `ShaderOverride`, dan `CommandList` sebagai ordered command-list sections. Karena urutan, kondisi, dan duplicate keys dapat bermakna, semantic fingerprint tidak boleh dibangun dari set header/hash mentah saja. Lihat [3DMigoto IniHandler](https://github.com/bo3b/3Dmigoto/blob/master/DirectX11/IniHandler.cpp) dan [upstream d3dx.ini](https://github.com/bo3b/3Dmigoto/blob/master/Dependencies/d3dx.ini). XXMI juga mengecualikan `DISABLED*` secara recursive saat menyiapkan runtime mod; lihat [XXMI model importer](https://github.com/SpectrumQT/XXMI-Launcher/blob/main/src/xxmi_launcher/core/packages/model_importers/model_importer.py).

## Penyebab utama kondisi saat ini

1. `dup_scan_start` menghitung progress memakai disk walker, tetapi pipeline dedup membangun kandidat baru dari row `mods` di DB. Child row yang seharusnya dimiliki root terminal dapat kembali masuk sebagai kandidat, sedangkan progress dan hasil memakai populasi berbeda.
2. `apply_modpack_filter` hanya mengecualikan dua folder yang memiliki immediate parent yang sama. Ia tidak memiliki konsep owner root, sehingga nested subvariant dan child hasil proyeksi DB masih dapat dibandingkan.
3. `merged.ini` belum dipahami sebagai orchestrator. Parser dedup hanya membaca 200 baris, menyimpan komentar/header sebagai signal, dan mengumpulkan hash tanpa tipe, namespace, condition, priority, atau resource binding.
4. Partial hash texture besar dipakai sebagai hasil akhir. Requirement AC-32.1.2 meminta full BLAKE3 confirmation, tetapi pass konfirmasi tersebut belum ada.
5. Scoring mencampur kemiripan nama/struktur dengan identitas konten. Hasil similarity dapat memperoleh aksi destruktif walaupun sebenarnya recolor, subvariant, atau mod yang menarget object sama.
6. Grouping union-find menggabungkan rantai `A~B` dan `B~C` lalu memakai score maksimum sebagai confidence seluruh group. Ini dapat membuat `A` dan `C` tampak pasti duplicate walaupun tidak pernah terverifikasi.
7. Hardlink resolver memeriksa ukuran file saja sebelum mengganti file. Ukuran sama bukan bukti konten sama.
8. Command start selesai segera setelah background task dibuat. Frontend meng-invalidasi report terlalu awal, sementara failure dikirim sebagai `Cancelled` dan report hanya disimpan di memory global, bukan per game/job.

## Istilah domain yang dipisahkan

| Istilah | Makna | Aturan dedup |
| --- | --- | --- |
| `LogicalModUnit` | Satu root mod yang dapat dimuat/dioperasikan sebagai satu kesatuan | Satu-satunya kandidat folder-level |
| `OrchestratedSubvariant` | Resource/branch internal yang dipilih oleh `merged.ini` | Tidak pernah menjadi kandidat duplicate tersendiri |
| `ToggleVariant` | Sibling variant yang dapat di-enable/disable sebagai pilihan terpisah | Tidak dibandingkan dengan sibling milik owner yang sama |
| `IndependentMod` | Root mod di luar ownership unit lain | Dapat dibandingkan dengan unit independen lain |
| `ExactCopy` | Manifest unit sama setelah full BLAKE3 | Boleh menawarkan Trash; hardlink sesuai policy |
| `SharedAssets` | Sebagian heavy asset sama, tetapi unit tidak identik | Hanya per-file optimization yang terverifikasi; bukan delete folder |
| `RelatedVariant` | Runtime target sama, resource/branch berbeda | Informational; tidak ada aksi storage destruktif |
| `RuntimeConflict` | Target/constraint runtime overlap | Diteruskan ke conflict workflow, bukan dilabeli duplicate |

## Opsi desain

### A. Patch parent filter

Pertahankan kandidat dari DB, lalu cari common ancestor lebih dalam dan tambahkan beberapa signal `merged.ini`.

- Kelebihan: perubahan kecil.
- Kekurangan: DB drift tetap memengaruhi hasil, ownership tersebar di beberapa fungsi, dan false positive baru akan muncul pada layout lain.

### B. Logical ownership + two-track matching — dipilih

Bangun satu inventory dari disk walker/classifier yang sama dengan workspace. Setiap candidate memiliki `owner_root`, `root_kind`, dan orchestration evidence. Scanner kemudian memisahkan exact-content matching dari semantic relation.

- Kelebihan: memperbaiki akar masalah; sesuai terminal-node invariant; aksi storage dapat diberi safety gate yang tegas.
- Kekurangan: memerlukan perubahan kontrak report dan test fixture yang lebih lengkap.

### C. Full 3DMigoto execution graph sejak awal

Parse seluruh INI menjadi AST dan simulasi graph override/command/resource sebelum hashing.

- Kelebihan: semantic accuracy tertinggi.
- Kekurangan: scope besar dan tidak diperlukan untuk menghentikan false duplicate saat ini.

Pendekatan B dipakai sekarang. Extractor semantic dibuat bertahap agar dapat berkembang ke C tanpa menahan bug fix ownership dan hash safety.

## Rancangan terpilih

### 1. Satu inventory, satu ownership boundary

- Disk walker menjadi sumber kandidat scan; row DB hanya melengkapi `mod_id`, safety state, dan whitelist.
- `ModPackRoot`, `FlatModRoot`, dan `VariantContainer` tetap terminal.
- Setiap descendant yang berada di bawah terminal root memperoleh `owner_root` yang sama dan tidak di-emitt sebagai candidate.
- Folder internal/staging, symlink, `DISABLED*` subtree yang memang tidak loadable, serta `.emmm-*` tidak menjadi unit mandiri.
- `merged.ini` hanyalah weak filename hint. Strong evidence adalah kombinasi root INI valid, cycle/key variable, conditional command list, dan `filename=`/resource references ke beberapa child directory.

### 2. Dua jalur matching

**Exact-content track**

- Ignore metadata/noise sesuai policy.
- Bucket berdasarkan ukuran dan extension agar murah.
- Partial BLAKE3 hanya prefilter.
- Semua calon match menjalani full BLAKE3 sebelum mendapat class `ExactCopy` atau `SharedAssets`.
- Folder identity memakai canonical content manifest; path-aware manifest tetap disimpan untuk membedakan exact layout dari renamed layout.

**3DMigoto semantic track**

- Reuse decoder/traversal INI yang sudah dipakai conflict scanner; hapus parser 200-line khusus dedup.
- Typed runtime target minimal: section kind, validated hash width, namespace, `match_first_index`, `match_priority`, condition context, command-list order, slot/checktextureoverride dependencies, cycle variable/value, resource binding, dan full content hash resource.
- Target sama + resource graph berbeda menghasilkan `RelatedVariant`/`RuntimeConflict`, bukan duplicate.
- Semantic score tidak pernah sendirian mengaktifkan Trash atau hardlink.

### 3. Grouping tanpa transitive certainty

- `ExactCopy` boleh digroup berdasarkan canonical manifest ID yang identik.
- `SharedAssets` disajikan sebagai asset clusters atau pair evidence, bukan union-find component dengan max score.
- `RelatedVariant` dan `RuntimeConflict` memiliki relation edges eksplisit; confidence satu edge tidak diwariskan ke semua member.

### 4. Persisted job dan lifecycle

- Gunakan tabel existing `dedup_jobs`, `dedup_groups`, dan `dedup_group_members`; tidak perlu migration pada tahap awal.
- Event state menjadi `Started -> Progress/Match -> Finished | Failed | Cancelled`.
- Report diambil berdasarkan `game_id` dan `scan_id`; frontend refresh hanya setelah terminal event.
- Scan baru tidak boleh menampilkan report game/job lama sebagai hasil baru.

### 5. Safe resolution

- Folder Trash hanya tersedia untuk `ExactCopy` unit yang masih cocok saat preflight.
- Hardlink hanya untuk file yang full hash-nya sama, berada pada volume yang mendukung, dan lolos revalidation tepat sebelum mutation.
- Mutation tetap memakai operation lock, watcher suppression, staging/rollback, Recycle Bin, dan trailing reconcile.
- Intra-owner subvariant tidak diberi aksi Trash/hardlink pada versi ini. Optimasi asset internal dapat menjadi fitur opt-in terpisah setelah ada fixture dan recovery UX.

## Acceptance criteria kunci

1. Root `merged.ini` dengan dua atau lebih child variant menghasilkan tepat satu `LogicalModUnit` dan nol duplicate pair internal, termasuk child nested lebih dari satu level.
2. Child source INI yang sudah menjadi `DISABLED*.ini` tetap dimiliki root merged dan tidak muncul sebagai candidate.
3. Dua salinan utuh dari merged mod pada root independen terdeteksi sebagai `ExactCopy` setelah full BLAKE3.
4. Dua mod dengan target hash sama tetapi resource berbeda diklasifikasi `RelatedVariant` atau `RuntimeConflict`, tanpa aksi delete/hardlink folder.
5. Dua file besar dengan head/tail sama tetapi middle bytes berbeda tidak pernah lolos sebagai exact duplicate.
6. Chain similarity tidak membentuk group exact kecuali setiap member memiliki canonical manifest ID yang sama.
7. Scan gagal menghasilkan `Failed`, scan cancel menghasilkan `Cancelled`, dan report baru terlihat setelah `Finished`.
8. Resolver menolak file yang berubah setelah scan dan tidak meninggalkan target hilang pada rollback.
9. Inventory dan progress memakai jumlah logical unit yang sama, termasuk pada DB yang masih memiliki child rows lama.
10. Scan 10.000 logical roots tetap bounded dan cancellation mencapai terminal state dalam target requirement.

## Non-goals tahap ini

- Tidak mengubah cara GIMI memilih variant saat runtime.
- Tidak mengedit isi `merged.ini` atau child INI.
- Tidak melakukan automatic/background dedup.
- Tidak menganggap shared target hash sebagai bukti duplicate.
- Tidak menawarkan cross-game dedup.
