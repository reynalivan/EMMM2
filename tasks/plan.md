# Rencana Perbaikan Gap 3DMigoto

## Tujuan

Menutup 22 gap terverifikasi di `.docs/3dmigoto_context_knowledge.md` tanpa membangun abstraction framework yang belum diperlukan. Hasil akhirnya harus membuat discovery, editor INI, KeyViewer, switch lifecycle, dan runtime 3DMigoto melihat state yang konsisten.

## Batasan implementasi

- Pertahankan filesystem sebagai source of truth.
- Gunakan raw INI lines sebagai basis render; tidak membuat AST/serializer INI baru.
- Tidak menambah dependency kecuali tidak ada primitive yang memadai di dependency saat ini.
- Tidak membuat `GameRuntimeProfile` generik. Gunakan helper kecil dan `match GameType` hanya pada boundary yang memang berbeda per package.
- Satu file source maksimal 350 baris; pecah berdasarkan tanggung jawab bila perlu.
- Perubahan scanner yang sedang dirty milik pengguna tidak boleh ditimpa. Gap INI-06 dan KV-10 dikerjakan setelah exact-diff checkpoint.
- Tidak mengklaim runtime reload berhasil bila aplikasi hanya menyelesaikan mutation disk/DB/artifact.

## Baseline

- Knowledge source: `.docs/3dmigoto_context_knowledge.md`.
- Baseline test yang sudah lulus: INI 12, KeyViewer 56, hotkeys 31.
- `.docs/history/` belum ada; log pertama dibuat sesudah implementasi selesai.
- Area yang akan disentuh pada fase awal bersih dari diff pengguna: `services/ini`, `services/keyviewer`, `services/hotkeys`, `common/normalizer.rs`, `repo/mod_repo`, dan `src/lib/disabledPrefix.ts`.
- Area scanner/master DB memiliki perubahan pengguna dan diperlakukan sebagai protected overlap.

## Dependency graph

```text
T1 runtime path/prefix contract
 |-- T2 recursive INI discovery
 |-- T3 reload binding discovery/replay
 |-- T4 effective-enabled projection
 |     `-- T7 KeyViewer artifacts/matcher
 `-- T8 archive/conflict completeness

T2 --> T5 lossless reader model --> T6 recoverable writer
T3 -------------------------------> T9 lifecycle/hotkey UX
T4 --> T7 --> T9
T6 + T7 + T8 + T9 --> T10 full verification
```

## T1 — Runtime path dan disabled-prefix contract

Gap: SW-01 dan foundation KV-03/KV-06.

Implementasi:

- Jadikan predicate runtime-disabled case-insensitive dan sesuai exclude upstream `DISABLED*` untuk setiap path component.
- Samakan predicate Rust dan TypeScript; canonical rename tetap menghasilkan `DISABLED `.
- Tambahkan helper kecil untuk memilih `d3dx.ini`: parent configured `mod_path` sebagai sumber utama, lalu parent `game_exe` hanya fallback kompatibilitas.
- Jangan menambahkan profile registry; helper menerima `GameConfig` yang sudah ada.

Acceptance criteria:

- `DISABLEDFoo`, `DISABLED_Foo`, `disabled-Foo`, dan `DISABLED Foo` efektif disabled di backend dan frontend.
- Enable menormalkan semua bentuk tersebut kembali ke nama bersih; disable selalu menghasilkan `DISABLED <name>`.
- Resolver memilih config di package/mod root pada layout XXMI dan tetap menemukan layout legacy dekat executable.

Verifikasi:

- Unit test normalizer Rust dan `disabledPrefix.ts`.
- Unit test resolver dengan temp layout package dan legacy.

## T2 — Discovery INI rekursif dan deterministik

Gap: INI-01 dan bagian error-discovery INI-05.

Implementasi:

- Ganti satu-level `read_dir` dengan walker rekursif berbasis stdlib.
- Jangan mengikuti directory symlink/reparse point.
- Lewati `desktop.ini` dan subtree dengan component runtime-disabled.
- Propagasi error directory entry; jangan memakai `flatten()`.
- Sort berdasarkan normalized relative path agar hasil stabil.

Acceptance criteria:

- Editor dan harvester melihat INI nested yang runtime lihat.
- File di subtree `DISABLED*` tidak dipanen.
- Unreadable/broken entry menghasilkan error yang dapat didiagnosis.
- Urutan output sama pada pengulangan scan.

Verifikasi:

- Fixture nested, mixed-case extension, disabled subtree, symlink loop, dan deterministic ordering.
- Jalankan test INI dan KeyViewer harvester.

## T3 — Reload binding upstream yang nyata

Gap: KV-01, KV-02, KV-03.

Implementasi:

- Parse `[Hunting] reload_fixes = ...` secara case-insensitive; hapus contract test fiktif `[Key*] type = reload_fixes`.
- Parser replay menerima token spasi atau `+`, `VK_*`, modifier positif, dan mengabaikan constraint negatif `no_*`/`no_modifiers`.
- Tolak binding tanpa tepat satu main keyboard key yang dapat direplay; fallback F10 hanya untuk config hilang/tidak memiliki binding.
- Gunakan satu resolver path bersama di command UI dan hotkey replay.

Acceptance criteria:

- `no_modifiers VK_F10` ditemukan dan direplay sebagai F10.
- `ctrl shift VK_F5` menekan dan melepas modifier dalam urutan aman.
- Binding ambiguous/controller-only gagal eksplisit, bukan mengirim key yang salah.
- UI dan in-game hotkey melaporkan binding dari file yang sama.

Verifikasi:

- Unit test parser Hunting, whitespace/comment/case, negative modifiers, VK names, dan invalid multi-main.
- Test path resolution layout GIMI/XXMI.
- Hotkey service tests.

## T4 — Effective-enabled sebagai satu predicate

Gap: KV-06 dan foundation SW-01/KV-10.

Implementasi:

- Tambahkan filter effective-enabled pada repository/service boundary yang mengembalikan mod runtime-active.
- Sebuah row enabled hanya efektif aktif bila tidak ada component path yang runtime-disabled.
- Gunakan helper yang sama untuk KeyViewer, enabled-name/path conflict input, dan projection terkait; jangan mengubah child DB status ketika ancestor ditoggle.

Acceptance criteria:

- Child `status=1` di bawah object root `DISABLED*` tidak masuk harvest/conflict/runtime list.
- Re-enable ancestor membuat child kembali efektif tanpa rewrite status child.
- Tidak ada duplikasi aturan prefix pada query caller.

Verifikasi:

- Repository/service test dengan matrix ancestor enabled/disabled dan legacy prefix.
- Integration test object-root toggle → query effective mods.

## T5 — Reader INI lossless tanpa AST baru

Gap: INI-02, INI-03, INI-04, sisa INI-05.

Implementasi:

- Simpan encoding asal (`Utf8`/`ShiftJis`/lossy), source fingerprint, serta terminator per raw line termasuk final/no-final newline.
- Pertahankan ordered raw lines; structured fields hanya index/view.
- Kenali qualifier `global`, `persist`, dan `local` sebelum `$variable`.
- Modelkan repeated `key`/`back` sebagai ordered assignments, sambil menjaga compatibility accessor yang dibutuhkan UI.
- Section identity dibandingkan case-insensitive tanpa mengubah ejaan asli.

Acceptance criteria:

- Parse → no-op render identik byte-for-byte untuk UTF-8/BOM, Shift-JIS, LF, CRLF, mixed terminators, dan final/no-final newline.
- Semua qualified variable dan repeated binding tampil dalam urutan source.
- Malformed/lossy input tetap read-only.

Verifikasi:

- Golden fixtures sesuai matrix acceptance knowledge doc.
- Update generated Specta binding hanya bila public DTO benar-benar berubah.

## T6 — Writer stale-safe dan recoverable

Gap: WRT-01, WRT-02, WRT-03.

Implementasi:

- Validasi seluruh line update sebelum side effect filesystem.
- Per-file in-process lock mencegah dua save EMMM bersamaan.
- Re-read target lalu bandingkan source fingerprint; external edit menghasilkan conflict.
- Encode kembali memakai encoding/BOM/terminator source dan tolak karakter yang tidak representable pada Shift-JIS.
- Tulis ke unique temp dalam directory yang sama.
- Commit dengan backup generation unik; bila replace gagal, restore original otomatis dan laporkan kedua error bila recovery juga gagal.

Acceptance criteria:

- Stale document tidak menimpa edit eksternal.
- Concurrent saves diserialisasi dan save kedua harus melewati stale check.
- Kegagalan commit selalu meninggalkan target asli atau backup yang jelas dapat dipulihkan.
- Targeted update tidak mengubah byte lain di file.

Verifikasi:

- Unit/integration tests no-op, targeted update, stale edit, concurrent save, unrepresentable Shift-JIS, dan injected commit failure.
- INI full test suite.

## T7 — KeyViewer portable, deterministik, dan fail-loud

Gap: KV-04, KV-05, KV-07, KV-08, KV-09.

Implementasi:

- Scope harvester resource match ke `TextureOverride` 8-hex; shader 16-hex dipanen sebagai type terpisah hanya untuk conflict scan, bukan dipaksa masuk matcher resource.
- Dedupe code hashes per object/skin sebelum scoring.
- Matcher hanya menghasilkan artifact bila memiliki sentinel yang unik terhadap object result lain; ambiguity ditandai dan dilewati.
- Ganti hardcode legacy GIMI help bridge dengan renderer kecil per `GameType`/capability yang memakai API text resmi package. Unsupported capability menghasilkan warning eksplisit dan tidak menulis INI rusak.
- Canonicalkan semua resource path relatif terhadap `.emmm_data/KeyViewer.ini`.
- Stage file baru, validasi, lalu replace; cleanup/create/write errors dipropagasi. Hapus fallback/dead resource path.

Acceptance criteria:

- Duplicate hashes tidak menaikkan score.
- Dua object tidak pernah memiliki sentinel output yang sama.
- GIMI/WWMI/SRMI/ZZMI memakai namespace/API yang sesuai atau menonaktifkan overlay dengan diagnosis jelas.
- Semua generated resource resolve dari lokasi config yang sama.
- Kegagalan artifact tidak dilaporkan sebagai refresh sukses.

Verifikasi:

- Matcher fixtures duplicate/shared/zero-sentinel/ambiguous.
- Golden INI per package dan resource-resolution integration test.
- Failure test directory/create/write/replace/cleanup.

## T8 — Archive classification dan conflict evidence

Gap: INI-06, KV-10. Area ini overlap dengan perubahan scanner pengguna.

Checkpoint wajib sebelum edit:

- Re-read exact diff pada file scanner/archive/master-db.
- Patch hanya fungsi klasifikasi/evidence yang tidak menghapus atau mereformat perubahan pengguna.
- Jika overlap semantik tidak bisa dipisah, berhenti dan minta keputusan pengguna.

Implementasi:

- Root classification menerima mod valid dengan Include/Key/CommandList/ShaderRegex walau tanpa TextureOverride/Resource.
- Conflict evidence menyimpan section type, namespace, condition, `match_first_index`, priority, dan hash type/provenance yang tersedia.
- Bedakan potential shared hash dari definite override conflict; ShaderFixes replacement diberi kategori sendiri bila datanya tersedia.

Acceptance criteria:

- Fixture key-only/include-only/shader-regex support mod tidak ditolak.
- Report tidak menyatakan definite conflict hanya karena dua file berbagi hash dengan conditions berbeda.
- Existing scan-review changes milik pengguna tetap utuh.

Verifikasi:

- Targeted archive/deepmatch/conflict fixtures.
- Exact `git diff` review sebelum dan sesudah patch.

## T9 — Switch recovery dan runtime-status UX

Gap: SW-02, SW-03, SW-04.

Implementasi:

- Ubah default global hotkey ke chord bermodifier yang tidak sama dengan upstream `no_modifiers F6/F8`; existing user config tidak dimigrasi diam-diam.
- Tambahkan conflict check terhadap reserved Hunting/package bindings yang berhasil ditemukan.
- Bila rollback rename menghasilkan warning/failure, jalankan full reconcile yang sudah ada dan kembalikan recovery warning terstruktur.
- UI membedakan disk/artifact applied dari `reload required`; in-game path boleh menandai reloaded hanya setelah key send sukses.

Acceptance criteria:

- Default baru tidak identik dengan F6/F8 package action.
- Partial rename/rollback failure memicu reconcile dan memberi recovery action yang terlihat.
- App toggle tidak pernah mengklaim runtime loaded; user selalu mendapat reload key/instruction.

Verifikasi:

- Hotkey default/conflict tests.
- Failure-injection batch rename → reconcile assertion.
- Frontend tests untuk applied/reload-required copy dan state.

## T10 — Verification dan dokumentasi akhir

Implementasi:

- Jalankan formatter/lint/typecheck dan seluruh targeted Rust/TS suites.
- Jalankan full test/build yang feasible; laporkan pre-existing failure terpisah.
- Audit semua 22 gap terhadap implementation + tests.
- Update `.docs/3dmigoto_context_knowledge.md` dengan status resolved/deferred dan alasan berbasis evidence.
- Buat `.docs/history/<timestamp>-3dmigoto-gap-remediation.md` sesuai `post_log.md`.

Acceptance criteria:

- Setiap ID memiliki status, file implementasi, dan test evidence.
- Tidak ada file source baru/melewati 350 baris.
- Tidak ada unrelated user change yang dihapus atau direformat.
- Build/test result dicatat apa adanya.

## Urutan checkpoint

1. Approval rencana.
2. T1–T4: runtime truth; targeted tests hijau.
3. T5–T6: reader/writer; golden dan failure tests hijau.
4. T7: KeyViewer; package fixtures hijau.
5. T8 exact-diff approval hanya bila overlap semantik ditemukan.
6. T9–T10: lifecycle, full verification, knowledge/history update.

Setelah setiap checkpoint, `tasks/todo.md` diperbarui sebelum lanjut agar scope tidak melebar.
