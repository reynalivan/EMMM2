# 3DMigoto Context Knowledge untuk EMMM2NEW

> Status: baseline riset per 2026-08-10. Dokumen ini merangkum percakapan Claude, kode EMMM2NEW saat ini, source code/repository upstream, dan dokumentasi modding yang diberikan. Gunakan sebagai konteks desain; temuan bertanda **runtime-test** belum boleh dianggap terbukti di semua fork XXMI.

## 1. Tujuan dan batas sumber

Fokus EMMM adalah empat subsistem yang saling terhubung:

1. INI Reader/Formatter: membaca file mod tanpa merusak bentuk aslinya dan mengekstrak variable/key binding yang dapat diedit.
2. INI Writer: mengubah baris terpilih dengan backup dan replace yang aman.
3. KeyViewer: memanen hash/keybind mod aktif, mencocokkannya dengan MasterDB, lalu membuat overlay 3DMigoto.
4. Switch mod: mengaktifkan/nonaktifkan folder melalui konvensi `DISABLED `, merekonsiliasi disk/DB, memperbarui artifact overlay, lalu memuat ulang konfigurasi runtime bila aman.

Hierarki kepercayaan sumber:

- Utama: source code [bo3b/3Dmigoto](https://github.com/bo3b/3Dmigoto), file konfigurasi paket XXMI, dan kode EMMM lokal.
- Pendukung: README, guides, releases, dan asset database milik pemilik project terkait.
- Tutorial: [leotorrez modding docs](https://leotorrez.github.io/modding/docs/); berguna untuk workflow, tetapi halaman itu sendiri menyatakan sintaksnya berorientasi GIMI/XXMI dan tidak semuanya berlaku pada upstream 3Dmigoto.
- `https://www.3dmigoto.com`: hasil scrape tidak dipakai sebagai sumber pengetahuan. README resmi bo3b memperingatkan domain tersebut adalah phishing dan bukan milik project; snapshot yang sempat diambil oleh research worker hanya berupa overview/marketing dan tidak mengalahkan peringatan tersebut. Rujukan resmi tetap [GitHub bo3b](https://github.com/bo3b/3Dmigoto).
- Context7: pencarian `3Dmigoto` menghasilkan “No libraries found”; Context7 bukan sumber yang cocok untuk project ini. Source GitHub menjadi fallback utama.

Percakapan Claude yang dibaca: `E:/Downloads/3dmigoto_EMMM2NEW.md`. Workflow deep-research terakhir di percakapan itu gagal seluruhnya karena limit sesi, sehingga kesimpulan lama diverifikasi ulang di sini dan tidak diterima mentah-mentah.

## 2. Peta ekosistem repository

| Repository | Isi penting | Posisi dalam sistem |
|---|---|---|
| [bo3b/3Dmigoto](https://github.com/bo3b/3Dmigoto) | Wrapper/hook DirectX 11, parser INI, command-list engine, hunting/frame analysis, loader, contoh `Dependencies/d3dx.ini` | Upstream resmi dan sumber semantik runtime |
| [rayshire/3dmigoto](https://github.com/rayshire/3dmigoto) | Fork `SilentNightSound/GI-Model-Importer`; root berisi Guides, Tools, dan ZIP GIMI | Bukan fork langsung upstream bo3b; mirror/varian GIMI lama |
| [SilentNightSound/GIMI-Package](https://github.com/SilentNightSound/GIMI-Package) | `GIMI/d3dx.ini`, `Core/GIMI`, shader/font/API/notification | Paket GIMI modern untuk XXMI; relevan untuk kontrak runtime saat ini |
| [SilentNightSound/GI-Model-Importer](https://github.com/SilentNightSound/GI-Model-Importer) | Guides, Blender tools/scripts, ZIP dev/play | Standalone GIMI lama; README menyatakan deprecated dan mengarahkan ke XXMI |
| [GI-Model-Importer-Assets](https://github.com/SilentNightSound/GI-Model-Importer-Assets) | Player/NPC/enemy/weapon/skill data, buffer/texture, `hash.json` | Sumber fakta hash/model GI untuk MasterDB/importer |
| [SR-Model-Importer](https://github.com/SilentNightSound/SR-Model-Importer) | Port importer/tools/guides untuk HSR | Tooling/export-import HSR berbasis pola GIMI |
| [SR-Model-Importer-Assets](https://github.com/SilentNightSound/SR-Model-Importer-Assets) | Player/weapon/skill/enemy assets dan `hash.json` | Sumber fakta hash/model HSR |
| [SpectrumQT/WWMI-Package](https://github.com/SpectrumQT/WWMI-Package) | `WWMI/d3dx.ini`, `Core/WWMI`, custom shader examples | Paket XXMI untuk Wuthering Waves; namespace/API tidak boleh diasumsikan sama dengan GIMI |
| [leotorrez/ZZ-Model-Importer](https://github.com/leotorrez/ZZ-Model-Importer) | ZIP ZZMI, Tools, petunjuk XXMI | Varian untuk ZZZ; hot-load F10 dan toggle paket F6 |
| [SpectrumQT/XXMI-Launcher](https://github.com/SpectrumQT/XXMI-Launcher) | Python app, core/gui, locale, themes, installer/updater | Orkestrator instalasi/launch/config; runtime mod tetap paket 3Dmigoto per game |

Relasi sederhananya: upstream 3Dmigoto menyediakan interception dan bahasa INI; paket GIMI/SRMI/WWMI/ZZMI mengadaptasi runtime per game; asset repos menyediakan hash dan data model; tools Blender menghasilkan resource/INI; XXMI Launcher memasang dan menjalankan paket tersebut; EMMM mengelola folder mod dan artifact runtime di atas layout `Mods`.

### 2.1 Tur file-level: bo3b/3Dmigoto

Root upstream memperlihatkan batas-batas tanggung jawab berikut:

- `DirectX11/`: wrapper device/context/resource dan titik utama interception DX11. Ini area yang membuat override dapat bereaksi terhadap draw/resource aktif.
- `DirectXGI/`: factory, swap-chain, dan present path. Overlay serta pekerjaan `[Present]` bergantung pada lifecycle ini.
- `DirectX9/` dan `DirectX10/`: komponen kompatibilitas lintas versi; tidak semua fitur fork XXMI dapat disimpulkan hanya dari label DX11.
- `Injector/`: loader/injection flow. Package XXMI dapat menaruh loader/runtime terpisah dari executable game.
- `HLSLDecompiler/`, `BinaryDecompiler/`, dan `D3D_Shaders/`: hunting, disassembly/decompilation, dan tooling shader.
- `D3DCompiler*`: wrapper/dukungan beberapa versi compiler shader.
- `Dependencies/`: template runtime, termasuk `d3dx.ini`, sumber praktis untuk grammar key, include, hunting, rendering, resource, dan loader.
- `NVAPI/`: integration NVIDIA/stereoscopic legacy; bukan core EMMM, tetapi menjelaskan asal project.
- `ini_parser_lite.cpp`, `vkeys.h`, dan source DirectX11 command list adalah rujukan ketika dokumentasi dan perilaku package berbeda.
- `StereovisionHacks.sln` adalah solution build upstream. Repository ini source developer, bukan distribusi end-user siap pakai.

README atau template `d3dx.ini` menjelaskan kontrak yang diharapkan, tetapi source parser/executor tetap otoritas terakhir untuk edge case seperti duplicate property, path resolution, namespace, dan pre/post ordering.

### 2.2 Tur file-level: paket runtime XXMI

#### GIMI-Package

Layout inti yang telah diverifikasi:

```text
GIMI/
├── d3dx.ini
└── Core/GIMI/
    ├── main.ini
    ├── API.ini
    ├── KeyBindings.ini
    ├── help.ini
    ├── d3dx_patch.ini
    ├── Fonts/
    ├── Libraries/
    ├── Notifications/
    └── Shaders/
```

- `d3dx.ini` memilih target/loader, memasukkan `Core\GIMI\main.ini`, dan memindai `Mods` rekursif.
- `main.ini` adalah composition root GIMI: subsystem core, renderer text, notification, dan variables package bertemu di sana.
- `API.ini` adalah contract publik untuk fitur seperti `CommandList\GIMIv8\PrintText`.
- `KeyBindings.ini` mendefinisikan F6/F10/F12 dan variable package.
- `help.ini` menyediakan bridge untuk mod lama, tetapi komentarnya menyatakan deprecated.
- `d3dx_patch.ini` adalah patch/config extension package dan bukan mod user biasa.
- Fonts/Shaders/Notifications adalah dependency renderer; hardcode satu file tanpa memahami dependency graph berisiko pecah saat package berubah.

#### WWMI-Package

Layout core yang telah diverifikasi:

```text
WWMI/
├── d3dx.ini
└── Core/WWMI/
    ├── WuWa-Model-Importer.ini
    ├── WWMI-Utilities.ini
    ├── KeyBindings.ini
    ├── help.ini
    ├── Fonts/
    ├── Notifications/
    └── Shaders/
```

Perbedaannya bukan kosmetik. Namespace `WWMIv1`, composition root, utilities, compatibility mode, dan API command list dapat berbeda dari `GIMIv8`. `GameType -> RuntimeProfile` harus memilih capability, bukan sekadar mengganti string nama game.

#### SRMI dan ZZMI

- Repository standalone SRMI/ZZMI menyimpan ZIP development/play dan tools, bukan source tree package modern selengkap GIMI-Package/WWMI-Package.
- SRMI README menjelaskan banyak file di-port dari GIMI dan mungkin masih menyimpan referensi Genshin; kompatibilitas tidak boleh diasumsikan hanya karena struktur serupa.
- ZZMI mengarahkan instalasi ke XXMI Launcher, memakai F6 untuk toggle dependent mods, F10 untuk reload/save, dan F12 untuk guide.
- Profile SRMI/ZZMI sebaiknya diturunkan dari instalasi package aktual yang dipilih user, bukan dari ZIP lama di repository saja.

### 2.3 Tur file-level: importer tools

`GI-Model-Importer/Tools` memberi gambaran pipeline authoring:

- `blender_3dmigoto_gimi.py`: addon import/export Blender dengan custom properties 3dmigoto.
- `genshin_3dmigoto_collect.py`: tooling collect/dump generasi lama.
- `genshin_3dmigoto_generate.py`: menghasilkan buffer/resource dan memformat INI hasil export.
- `genshin_merge_mods.py`: menggabungkan mod; hasilnya dapat memiliki nested INI dan struktur lebih kompleks daripada satu mod sederhana.
- `genshin_animation_creator.py`, damage merge, outline, transparency, dan color scripts menunjukkan INI mod dapat mempunyai command list/variable kaya.
- Script Blender untuk bone deletion, vertex-group remap/merge/fill, unused-group cleanup, dan custom-property transfer menjaga metadata model, tetapi berada di luar tanggung jawab INI editor EMMM.

File mod valid dapat dibuat manusia, generator, atau merge tool. Reader/Writer EMMM tidak boleh memformat ulang seluruh file berdasarkan satu gaya contoh.

### 2.4 Tur file-level: asset repositories

GI assets membagi data menjadi `PlayerCharacterData`, `NPCData`, `EnemyData`, `WeaponData`, `SkillData`, dan `MiscellaneousData`. SR assets memakai `PlayerCharacterData`, `WeaponData`, `SkillObjData`, dan `EnemyData`.

Satu folder object dapat memuat model/buffer/texture dan `hash.json`. Bentuk `hash.json` adalah array component, bukan satu flat object:

| Field | Makna praktis | Karakteristik |
|---|---|---|
| `component_name` | Nama bagian seperti Hair/Body/Face atau kosong untuk component utama | Tidak selalu unik/global |
| `root_vs` | Root vertex shader | Umumnya 16 hex; dapat kosong |
| `draw_vb` | Hash buffer pengenal draw/model | 8 hex |
| `position_vb` | Posisi vertex | 8 hex |
| `blend_vb` | Weight/bone blend | 8 hex |
| `texcoord_vb` | UV/tangent/normal stream sesuai game | 8 hex |
| `ib` | Index buffer | 8 hex |
| `object_indexes` | First-index/slice component | Array integer |
| `object_classifications` | Label bagian per index | Array sejajar dengan indexes |
| `texture_hashes` | Texture type, extension, hash per classification | Nested array; hash 8 hex |

Albedo mempunyai component utama dengan `root_vs` 16-hex dan component `Face` terpisah yang sebagian field-nya kosong. Blade di SR mempunyai Hair, Head, dan Body dengan beberapa classification. Loader MasterDB harus menjaga provenance game/object/component/skin/hash-type; flatten tanpa type menghilangkan informasi penting bagi matcher.

### 2.5 Tur file-level: XXMI Launcher

Root launcher berisi `src/xxmi_launcher`, `Locale`, `Themes/Default`, `public-media`, dan `requirements.txt`. Tanggung jawabnya:

- memasang package model importer yang benar;
- menyimpan konfigurasi launch dan path;
- memperbarui launcher serta instance package;
- memverifikasi authenticity download/library menurut README;
- memulai game/runtime dengan layout yang mungkin berbeda dari folder executable game.

EMMM tidak seharusnya menganggap `game_exe.parent()` adalah root package. `mod_path`, `loader_exe`, dan discovery instalasi XXMI aktual lebih kuat sebagai sumber lokasi runtime.

### 2.6 Matriks status sumber

| Sumber | Kegunaan | Caveat |
|---|---|---|
| bo3b master/source | Semantik upstream dan template konfigurasi | Fork XXMI membawa patch/extension sendiri |
| GIMI-Package/WWMI-Package | Kontrak package modern | Berubah mengikuti update package; perlu version detection |
| Standalone GI/SR/ZZ importer | Guides dan tools authoring | Sebagian deprecated/outdated; bukan selalu runtime modern |
| Asset repositories | Ground truth hash/model per commit | Hash dapat usang setelah update game; perlu commit/version metadata |
| Leotorrez docs | Workflow modder dan contoh sintaks | GIMI-oriented dan mengandung penyederhanaan |
| Percakapan Claude | Hipotesis dan arah audit | Deep-research gagal; semua temuan perlu verifikasi ulang |

## 3. Mental model runtime 3Dmigoto

3Dmigoto membungkus/meng-hook jalur DirectX, mengamati resource dan draw call, lalu menjalankan command list yang dikompilasi dari INI pada event seperti shader/resource match dan `Present`. Ini bukan parser INI generik: section tertentu adalah konfigurasi biasa, sedangkan `TextureOverride*`, `ShaderOverride*`, `CommandList*`, `CustomShader*`, dan beberapa section global membentuk program runtime.

Alur dasar:

```text
d3dx.ini -> [Include] -> include file/pohon Mods -> parse + namespace
         -> Texture/ShaderOverride cocok pada draw/resource
         -> command list pre/post mengubah state/resource/draw
         -> Present menjalankan pekerjaan per-frame/overlay
         -> F10 reload_fixes + reload_config memuat perubahan runtime
```

Fakta konfigurasi utama dari [upstream d3dx.ini](https://github.com/bo3b/3Dmigoto/blob/master/Dependencies/d3dx.ini) dan [GIMI d3dx.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/d3dx.ini):

```ini
[Include]
include = Core\GIMI\main.ini
include_recursive = Mods
exclude_recursive = DISABLED*
exclude_recursive = desktop.ini

[Hunting]
reload_fixes = no_modifiers VK_F10
reload_config = no_modifiers VK_F10
analyse_frame = no_modifiers VK_F8
```

Implikasi untuk EMMM:

- Disable adalah aturan filesystem yang dievaluasi saat include traversal. Rename folder adalah mutasi sumber kebenaran; DB hanya proyeksi.
- `DISABLED*` lebih luas daripada `DISABLED `: semua nama yang dimulai kata itu dapat di-exclude oleh runtime.
- F10 diperlukan untuk hot-load konfigurasi/INI baru. Tidak semua setting dapat direload tanpa restart.
- Binding 3Dmigoto dipisahkan spasi, mendukung positive/negative modifiers (`ctrl`, `no_shift`, `no_modifiers`) dan nama `VK_*`; ini berbeda dari format UI global-hotkey EMMM yang memakai `Shift+F6`.
- GIMI dan WWMI memakai F6 untuk toggle semua mod paket; F8 dipakai frame analysis. Default EMMM F6/F8 bertabrakan secara semantik.

### 3.1 Taxonomy section runtime

| Section/prefix | Peran | Implikasi bagi parser/editor |
|---|---|---|
| `[Include]` | Memilih file atau pohon INI tambahan dan exclusion | Menentukan universe file aktif; harus menjadi input discovery policy |
| `[Loader]` | Target, loader, module, elevation/launch behavior | Membantu menemukan package root tetapi tidak aman untuk diedit sebagai mod biasa |
| `[Hunting]` | Hunting state, marking, cycle keys, F8/F10, dump options | Grammar binding khusus 3dmigoto, bukan `[Key*]` |
| `[Rendering]` | Hash mode, shader/cache directories, resource tracking | Perubahan dapat memerlukan restart atau invalidasi cache |
| `[Constants]` | Global/persistent variables dan initialization command | Declaration qualifier serta ordering harus dipertahankan |
| `[Key*]` | Hold/toggle/cycle, repeated `key`, `back`, condition, command list | One-key struct tidak cukup; multiple binding valid |
| `[TextureOverride*]` | Match resource/buffer hash dan jalankan command/draw/resource replacement | Hash umumnya 8-hex; section executable dan order-sensitive |
| `[ShaderOverride*]` | Match shader dan jalankan override/filter/command | Hash umumnya 16-hex; tidak boleh dipaksa ke type resource32 |
| `[Resource*]` | Deklarasi buffer/texture/file/ref/data | Path, format, bind flags, dan namespace penting |
| `[CommandList*]` | Ordered executable commands dan flow control | Duplicate command adalah sequence, bukan conflict key |
| `[CustomShader*]` | Pipeline shader/render state custom | Bisa merujuk file shader dan resource eksternal |
| `[Present]` | Per-frame driver setelah/seputar present | Beberapa included file dapat berkontribusi; ordering dan pre/post perlu runtime test |

### 3.2 Grammar binding 3dmigoto

Contoh nilai yang valid/umum:

```ini
key = F6
key = no_modifiers F6
key = CTRL F12
reload_fixes = no_modifiers VK_F10
wipe_user_config = ctrl alt no_shift VK_F10
next_marking_mode = no_modifiers VK_DECIMAL VK_NUMPAD0
key = XB_X
```

Parser target seharusnya menghasilkan model semantik, misalnya:

```text
required modifiers   = {Ctrl, Alt}
forbidden modifiers  = {Shift}
main chord keys      = {F10}
controller binding   = optional XB_*
source spelling      = tetap disimpan untuk round-trip
```

`no_modifiers` bukan tombol yang perlu ditekan, melainkan shorthand constraint. `VK_` adalah prefix nama virtual key yang boleh ada atau tidak. Beberapa hunting binding membentuk chord lebih dari satu non-modifier key, sehingga fungsi “semua token kecuali terakhir adalah modifier” juga belum cukup umum.

### 3.3 Lifecycle reload dan persistence

- `reload_fixes` membaca ulang shader fixes; `reload_config` membaca ulang konfigurasi/INI. Package lazimnya mengikat keduanya ke F10.
- Persistent variables dapat ditulis ke `d3dx_user.ini`; `wipe_user_config` menghapus state tersebut lalu reload.
- Rename folder mengubah hasil include traversal pada reload berikutnya, bukan menjamin runtime yang sudah berjalan langsung berhenti memakai command list lama.
- Reload dapat mahal saat banyak shader/INI. GIMI modern mempunyai opsi delay/cache; EMMM perlu debounce dan status progress, bukan spam F10.
- Beberapa setting `d3dx.ini` tidak reloadable dan tetap memerlukan restart. EMMM harus menyebut “reload requested” alih-alih menjamin semua runtime state berubah.

### 3.4 Kapan sebuah mod efektif aktif

Sebuah child mod efektif aktif hanya jika seluruh kondisi ini benar:

```text
folder child tidak cocok exclusion
AND setiap ancestor sampai Mods root tidak cocok exclusion
AND file INI termasuk oleh include policy
AND config terbaru sudah dimuat runtime
AND condition/variable package mengizinkan command list
```

Status `mods.status = 1` hanya menjawab prefix child pada proyeksi DB. Ia belum membuktikan effective activation. Ini alasan `EffectiveEnabledResolver` perlu menjadi primitive bersama, bukan logic tambahan khusus KeyViewer.

## 4. Semantik INI yang harus dipertahankan

### 4.1 Struktur dan namespace

- Nama section/property tidak peka huruf besar-kecil.
- `namespace = ...` dapat mengubah namespace file. Referensi lintas namespace memakai bentuk seperti `Resource\GIMIv8\Text`.
- File yang di-include dapat mempunyai namespace berdasarkan file/path; section global seperti `[Present]` dan `[Constants]` dapat berkontribusi pada runtime global/tergabung, sehingga collision dan urutan eksekusi penting.
- Command list mengizinkan urutan perintah, repeated command, flow control `if`/`else if`/`else`/`endif`, serta fase `pre`/`post`. Model `Map<String,String>` biasa tidak cukup untuk representasi lossless.
- Upstream mendukung beberapa `key =` pada satu `[Key*]` dan `back =` untuk reverse cycle.
- Komentar yang aman adalah whole-line `; ...`; tutorial Leotorrez memperingatkan inline comment dapat menimbulkan error kompilasi meski syntax highlighter menerimanya.

### 4.2 Hash dan resource

- Hash shader lazimnya 16 hex/64-bit, misalnya `root_vs = 653c63ba4a73ca8b`.
- Hash buffer/texture lazimnya 8 hex/32-bit, misalnya `ib = 0d7dc936` atau texture `9bd05da3`.
- Contoh [Albedo `hash.json`](https://raw.githubusercontent.com/SilentNightSound/GI-Model-Importer-Assets/main/PlayerCharacterData/Albedo/hash.json) menyimpan `root_vs`, draw/position/blend/texcoord VB, IB, index/classification, dan texture hashes per component.
- `TextureOverride` tidak identik dengan “texture file saja”; pada importer, IB/VB juga sering dicocokkan melalui mekanisme resource override.
- `filename` resource harus diperlakukan relatif terhadap konteks file INI/package yang memuatnya; kontrak path harus dites dengan file yang benar-benar diletakkan pada posisi produksi.

Upstream resolver mencoba path relatif terhadap namespace/config file terlebih dahulu, lalu fallback relatif terhadap root 3dmigoto. Dua bentuk path dapat sama-sama terlihat bekerja tetapi melalui branch resolution berbeda. EMMM sebaiknya memilih satu kontrak eksplisit dan tidak bergantung pada fallback terselubung.

### 4.2.1 Match tidak cukup dinilai dari hash saja

`TextureOverride*` dapat mempersempit match dengan `match_first_index`, first vertex/instance, count, format/dimension/resource description, condition, dan `match_priority`. Dua section dengan hash sama belum tentu konflik bila draw context berbeda. Sebaliknya satu hash shared yang dipakai tanpa context dapat mengaktifkan KeyViewer untuk object yang salah.

`ShaderOverride*` dan `ShaderRegex*` juga dapat mengaktifkan `checktextureoverride` pada slot tertentu (`ps-t0`, `vb0`, `ib`). Karena itu “hash muncul di INI” tidak selalu berarti runtime mengevaluasi hash tersebut pada semua draw.

Kontrak evidence yang lebih kuat untuk MasterDB/KeyViewer:

```text
game + package version
object + component + skin
hash type/width
override section type
draw/resource context
source file/namespace
first-index atau constraints lain
confidence/provenance
```

### 4.3 Overlay text pada GIMI modern

[GIMI API.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/API.ini) menetapkan API yang didukung:

```ini
Resource\GIMIv8\Text = ref ResourceMyText
Resource\GIMIv8\TextParams = ref ResourceMyTextParams
run = CommandList\GIMIv8\PrintText
```

[GIMI help.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/help.ini) adalah compatibility layer deprecated. `CustomShaderFormatText` hanya dummy; rendering aktualnya meneruskan resource ke `CommandList\GIMIv8\PrintText`. Karena WWMI memakai namespace `WWMIv1`, EMMM membutuhkan adapter/capability per game/package, bukan hardcode GIMI.

## 5. Hunting, texture, dan shader workflow

Sumber: [Hunting](https://leotorrez.github.io/modding/guides/hunting), [Textures 101](https://leotorrez.github.io/modding/guides/textures-101), dan [Shaders 101](https://leotorrez.github.io/modding/guides/shaders-101).

1. Aktifkan hunting sebelum launch; untuk shader editing aktifkan dump shader.
2. Nonaktifkan mod yang dapat mengganggu target. F6 adalah toggle paket, tetapi shader tertentu dapat tetap aktif; rename ancestor dengan `DISABLED` lebih kuat.
3. Numpad 0 mengaktifkan hunting; cycle VS/PS/IB/VB/CS lalu mark/copy hash.
4. F8 membuat frame analysis. Dump global dapat berukuran sangat besar; targetkan resource melalui override/dump command bila memungkinkan.
5. `gui_collect` mengubah dump menjadi IB/VB/texture plus `hash.json`, kemudian Blender importer/exporter membentuk resource dan INI mod.
6. Texture replacement harus mempertahankan format/color space yang sesuai. Shader replacement memakai hash 16-hex, file replacement, resource slots, dan dapat menerima IniParams/time dari INI.
7. Hash shader lebih sering berubah setelah update game daripada beberapa hash model/resource; database dan matcher perlu versioning/provenance.

## 6. Implementasi EMMM saat ini

### 6.1 INI Reader/Formatter

- [`document.rs`](../src-tauri/src/services/ini/document.rs) menemukan INI langsung di satu folder, mengecualikan `desktop.ini`, membatasi 2 MiB, mendeteksi BOM/newline, mencoba UTF-8 lalu Shift-JIS, dan menyimpan `raw_lines`.
- Structured view hanya mengekstrak assignment `$variable = ...` yang dimulai `$`, serta satu `key` dan satu `back` per `[Key*]`.
- Malformed/lossy input masuk `RawFallback`; writer menolak menyimpannya.

### 6.2 INI Writer

- [`write.rs`](../src-tauri/src/services/ini/write.rs) membuat `.bak`, mengubah line index pada salinan `raw_lines`, mempertahankan BOM flag dan satu newline style, menulis `.tmp`, lalu rename.
- Ini cukup aman untuk edit kecil pada UTF-8 sederhana, tetapi belum lossless penuh dan belum aman terhadap concurrent/stale edits.

### 6.3 KeyViewer

- [`post_apply.rs`](../src-tauri/src/services/app/post_apply.rs) membaca mod berstatus enabled, memanen hash/keybind satu pass, memuat object/hash DB, mencocokkan, lalu menulis `.emmm_data/KeyViewer.ini`, `keybinds/active/*.txt`, dan status.
- [`harvester.rs`](../src-tauri/src/services/keyviewer/harvester.rs) hanya menerima hash 8-hex dari `TextureOverride*` atau `ShaderOverride*` dan memakai denylist nama section.
- [`matcher.rs`](../src-tauri/src/services/keyviewer/matcher.rs) memberi base/occurrence/rarity score, lalu memilih sampai tiga sentinel yang tidak high-collision.
- [`generator/ini.rs`](../src-tauri/src/services/keyviewer/generator/ini.rs) membuat TextureOverride sentinel, state per-frame, dan overlay melalui legacy `ShaderFixes\help.ini`.

### 6.4 Switch mod

- [`core_ops/toggle.rs`](../src-tauri/src/services/mods/core_ops/toggle.rs) dan [`runtime_mutation_engine.rs`](../src-tauri/src/services/runtime_mutation_engine.rs) melakukan rename filesystem dengan collision check, lock/suppression, batch plan, dan rollback best-effort.
- [`workspace_switch_service.rs`](../src-tauri/src/services/workspace_switch_service.rs) memisahkan object-root switch dari mod switch dan menjalankan scoped disk reconcile.
- Disk reconcile menyelesaikan DB/runtime projection dan memicu refresh artifact overlay bila folder berubah.
- UI switch tidak menyuntikkan F10 karena game mungkin tidak fokus; [`hotkey_cmds.rs`](../src-tauri/src/commands/app/hotkey_cmds.rs) hanya mengembalikan key yang harus ditekan. Cycle preset in-game dapat mengirim reload karena callback sudah memastikan game fokus.

### 6.5 Call flow Reader -> Editor -> Writer

```text
mod path
  -> list_ini_files (direct children saja)
  -> read_ini_document
       -> metadata size guard 2 MiB
       -> read bytes
       -> detect any CRLF
       -> UTF-8 / Shift-JIS / lossy decode
       -> split raw_lines
       -> parse $variable dan [Key*] key/back
  -> frontend membuat line_updates
  -> save_ini_with_updates
       -> reject RawFallback
       -> read current original untuk .bak
       -> patch cached raw_lines by index
       -> join memakai satu newline style
       -> write fixed .tmp
       -> rename/Windows remove+rename fallback
```

Invariant yang sudah baik:

- raw lines menjadi sumber rendering ulang, bukan serializer dari structured fields;
- save ditolak bila decode/parser tidak dipercaya;
- backup menyimpan bytes original saat save dimulai;
- target update divalidasi terhadap bounds.

Invariant yang belum ada:

- bytes saat save sama dengan bytes saat document dibaca;
- encoding/output sama dengan encoding/input;
- final newline dan mixed terminator identik;
- seluruh update valid sebelum side effect `.bak`/`.tmp`;
- satu writer per file dan replace selalu recoverable.

### 6.6 Call flow mutation -> reconcile -> overlay

```text
UI/workspace/collection/hotkey mutation
  -> operation lock + watcher suppression
  -> rename folder(s) di disk
  -> scoped disk reconcile
       -> scan/classify disk
       -> settle object/mod rows + runtime projection
       -> collection dirty-state bila perlu
       -> trigger_overlay_refresh_for_game
            -> rebuild projection
            -> query enabled child rows
            -> conflict scan
            -> hash/keybind harvest
            -> DB object/hash match
            -> KeyViewer.ini + text + status
  -> caller menginformasikan reload key atau hotkey path mereplay key
```

Design yang matang di alur ini:

- disk diposisikan sebagai source of truth;
- object-root switch dipisahkan dari mod-level switch;
- operation lock dan watcher suppression mengurangi race/feedback loop;
- batch mutation direncanakan sebelum rename dan memiliki rollback best-effort;
- reconcile menjadi single writer DB setelah filesystem berubah;
- runtime side effects diurutkan setelah projection settle.

Risiko boundary:

- query overlay kembali memakai raw `mods.status`, sehingga dapat membatalkan semantic effective-disabled yang projection/UI sudah pahami;
- beberapa runtime side-effect error dilog dan tidak dipropagasi karena disk mutation telah sukses. UX perlu menunjukkan degraded completion;
- artifact cleanup bukan transaction dan mengabaikan beberapa error filesystem;
- game runtime reload adalah boundary eksternal yang belum masuk transaction disk/DB/artifact.

### 6.7 Call flow KeyViewer secara detail

```text
get_enabled_mods_paths
  -> untuk setiap mod: list direct INI
  -> decode sekali
  -> harvest 8-hex override hashes
  -> parse [Key*]
  -> occurrence_counts + hash_to_mod_path
  -> get_kv_matching_objects
  -> flatten skin hashes menjadi code_hashes
  -> match_objects
       -> reverse hash -> object-name set
       -> intersection + score
       -> threshold filter
       -> sentinel rarity/collision filter
  -> generate TextureOverride per sentinel
  -> map sentinel kembali ke source mod keybinds
  -> write text resources dan status
```

Titik kehilangan informasi:

- file nested tidak masuk;
- type hash dan component provenance hilang saat flatten;
- repeated key/back hilang pada parser;
- object type di-hardcode `Character` pada pembentukan `KvObjectEntry`;
- tags dan thumbnail dikosongkan;
- sumber keybind dipetakan hanya melalui sentinel, bukan seluruh matched evidence;
- confidence berbasis kelipatan threshold, bukan probabilitas atau kalibrasi fixture.

### 6.8 Hotkey ownership

Ada tiga domain hotkey yang harus dipisahkan:

1. Global OS hotkey milik EMMM, diregister oleh Tauri dan ditulis user sebagai `Shift+F6`.
2. Key section di mod/paket, diproses 3dmigoto ketika game foreground dan ditulis `no_modifiers F6`.
3. Hunting key di `[Hunting]`, bukan `[Key*]`, dengan chord/negative modifier dan aksi engine seperti reload/frame analysis.

Konflik lintas domain tidak terdeteksi oleh `detect_conflicts`, karena fungsi lokal hanya membandingkan action EMMM satu sama lain. Runtime profile harus menyediakan reserved bindings, lalu scanner aktual menambah binding dari `d3dx.ini` dan included core package.

### 6.9 Arti hasil test saat ini

Validasi lokal yang dijalankan untuk baseline ini:

- `cargo test services::ini::`: 12 test lulus.
- `cargo test services::keyviewer::`: 56 test lulus.
- `cargo test services::hotkeys::`: 31 test lulus.

Kelulusan ini membuktikan implementasi konsisten dengan contract test saat ini, bukan bahwa contract tersebut cocok dengan upstream. Dua contoh contract test yang justru perlu diperbaiki:

- reload-key tests membuat `[KeyReload] type = reload_fixes`, sedangkan config upstream memakai `[Hunting] reload_fixes = ...`;
- KeyViewer generator tests secara eksplisit mengharuskan legacy `CustomShader\ShaderFixes\help.ini\FormatText` dan path `.emmm_data/status/...`, sehingga regression suite mempertahankan dua asumsi yang sedang diaudit.

## 7. Gap terverifikasi dan prioritas perbaikan

| ID | Prioritas | Gap terverifikasi | Dampak |
|---|---:|---|---|
| INI-01 | P0 | `list_ini_files` hanya satu level, sedangkan runtime memakai `include_recursive = Mods` | Nested INI tidak ikut editor, hash harvest, atau keybind harvest; EMMM dan runtime melihat dunia berbeda |
| INI-02 | P0 | Decoder Shift-JIS tidak menyimpan encoding asal; writer selalu menghasilkan UTF-8 | Save pada Shift-JIS “bersih” melakukan transcoding diam-diam |
| INI-03 | P1 | `text.lines()` + `join()` menghilangkan final newline dan menormalisasi mixed newline | Klaim lossless belum benar; diff dapat lebih luas dari satu baris |
| INI-04 | P1 | Structured model tidak mengenali `global/persist/local $var` dan hanya menyimpan key/back terakhir | Banyak variable resmi dan multiple bindings hilang dari view/edit |
| INI-05 | P1 | Section identity pada structured view belum case-insensitive dan error entry dari `read_dir` di-drop dengan `flatten()` | Duplicate logical section atau unreadable entry dapat disajikan keliru/tanpa diagnosis |
| WRT-01 | P0 | Tidak ada optimistic concurrency check terhadap bytes/mtime yang dibaca | External edit setelah read dapat ditimpa oleh stale `raw_lines` |
| WRT-02 | P0 | Windows fallback menghapus original sebelum rename kedua; kegagalan kedua tidak auto-restore `.bak` | Ada window file utama hilang walau backup masih tersedia |
| WRT-03 | P1 | Nama `.tmp` tetap dan tidak ada per-file lock; `.bak` ditimpa setiap save | Concurrent save dapat race; riwayat recovery hanya satu generasi |
| KV-01 | P0 | Reload discovery mencari `[Key*] type = reload_fixes`; upstream menaruh `reload_fixes` di `[Hunting]` | Selalu fallback untuk konfigurasi nyata; rebind user tidak dihormati |
| KV-02 | P0 | Parser pengiriman reload memakai format `+`; binding upstream memakai spasi, `no_*`, dan `VK_*` | Binding nyata seperti `no_modifiers VK_F10` gagal direplay |
| KV-03 | P0 | `d3dx.ini` dicari di `game_exe.parent()`; pada layout XXMI kontrak lebih kuat adalah parent dari configured `Mods`/package root | File konfigurasi yang dibaca dapat salah atau tidak ditemukan |
| KV-04 | P0 | Generator hardcode compatibility layer GIMI `ShaderFixes\help.ini`; `FormatText` sudah dummy/deprecated | Overlay rapuh dan tidak portable ke WWMI/ZZMI/SRMI |
| KV-05 | P0 | `KeyViewer.ini` berada di `.emmm_data`, status memakai root-style `.emmm_data/status/...`, sedangkan keybind memakai namespace-relative `keybinds/active/...` | Keduanya dapat bekerja karena resolver mencoba config-relative lalu root fallback, tetapi kontrak campuran rapuh dan wajib integration test |
| KV-06 | P0 | Enabled-mod query hanya `mods.status = 1`; child di bawah object-root `DISABLED ` sengaja tidak dicascade | KeyViewer/conflict scan dapat memanen mod yang efektifnya tidak dimuat 3Dmigoto |
| KV-07 | P1 | Harvester mengiklankan ShaderOverride tetapi regex hanya 8-hex | Shader 16-hex selalu diabaikan; model data perlu typed hash atau scope hanya resource override |
| KV-08 | P1 | `code_hashes` hasil flatten belum dideduplicate; threshold 5 < satu base hit 10; sentinel dengan collision count 2 masih dapat dimiliki dua object | Score bisa terinflasi, match false-positive, sentinel/resource ambigu atau tertimpa |
| KV-09 | P1 | Cleanup/create directory menelan error; fallback resource dibuat tetapi tidak pernah dipilih | Artifact stale dan dead path dapat lolos tanpa diagnosis |
| SW-01 | P0 | Upstream mengecualikan `DISABLED*`, Rust mengenali delimiter space/underscore/dash, frontend hanya canonical whitespace | Status UI/DB/runtime berbeda untuk nama legacy/eksternal seperti `DISABLEDFoo` atau `DISABLED_Foo` |
| SW-02 | P0 | Default EMMM F6/F8 bertabrakan dengan F6 package toggle dan F8 frame analysis | Satu key dapat memicu dua tindakan atau menutup fitur hunting; default harus package-aware |
| SW-03 | P1 | Rollback batch bersifat best-effort; bila rollback gagal, error tidak membawa reconcile recovery plan | Disk/DB dapat drift setelah partial rename yang tidak pulih |
| SW-04 | P1 | UI mutation sengaja hanya menampilkan reload key, bukan memastikan runtime sudah reload | Status “applied” berarti disk/DB selesai, belum tentu game sudah memakai state baru |
| INI-06 | P1 | Archive/root classification lokal cenderung mengenali `TextureOverride`, `ShaderOverride`, atau `Resource`; mod valid dapat hanya berisi Include/Key/CommandList/ShaderRegex | Paket support atau shader/key-only berisiko salah ditolak/diklasifikasi |
| KV-10 | P1 | Conflict scan berbasis hash belum merekam `match_first_index`, condition, namespace, priority, atau ShaderFixes replacement | Shared hash dapat over-report; shader/global conflicts dapat under-report |

### 7.1 Status remediation EMMM2NEW (2026-08-10)

Catatan audit: tabel di atas memuat **23 ID unik**, walaupun roadmap awal menyebut 22. Status di bawah mencakup seluruh 23 ID.

| ID | Status | Bukti implementasi | Bukti verifikasi |
|---|---|---|---|
| INI-01 | Resolved | `services/ini/document.rs` melakukan discovery rekursif, deterministik, tidak mengikuti symlink, dan melewati subtree runtime-disabled; editor dan KeyViewer memakai walker yang sama | `document_tests::test_list_ini_files_recurses_deterministically_and_skips_disabled_subtrees`; KeyViewer suite |
| INI-02 | Resolved | `services/ini/encoding.rs` mempertahankan UTF-8/BOM, Shift-JIS, dan menolak write lossy/unrepresentable | INI golden/write tests |
| INI-03 | Resolved | Terminator disimpan per baris (`LF`, `CRLF`, `CR`, none), termasuk final-newline state | mixed-terminator dan final-newline tests |
| INI-04 | Resolved | Structured view menyimpan qualifier `global/persist/local` dan repeated key/back dalam source order | `document_tests` qualified/repeated fixtures |
| INI-05 | Resolved | Section identity canonical case-insensitive; walker mempropagasi entry error | INI document suite |
| WRT-01 | Resolved | DTO membawa BLAKE3 source fingerprint; writer membandingkan bytes terbaru sebelum commit | stale-write rejection tests |
| WRT-02 | Resolved | Commit memakai sibling temp + recovery rename dan auto-restore bila replace gagal | injected recovery/commit tests |
| WRT-03 | Resolved | `OperationLock` menyerialisasi command write, temp name unik, backup diputar tiga generasi | preview/write concurrency and rotation tests |
| KV-01 | Resolved | Reload dibaca dari `[Hunting] reload_fixes` | reload discovery fixtures |
| KV-02 | Resolved | Replay menerima whitespace/`+`, `VK_*`, modifier positif, dan mengabaikan `no_*` | reload grammar tests |
| KV-03 | Resolved | Resolver memilih parent configured `Mods`, lalu parent executable sebagai fallback | package/legacy path tests |
| KV-04 | Resolved | Generator memakai API resmi `GIMIv8`, `SRMIv1`, `WWMIv1`, `ZZMIv1`; EFMI unsupported gagal eksplisit | package matrix golden tests |
| KV-05 | Resolved in code | Semua resource generated relatif terhadap `.emmm_data/KeyViewer.ini` (`status/...`, `keybinds/active/...`) | generator resource-resolution tests; validasi visual in-game per package tetap disarankan |
| KV-06 | Resolved | Enabled repository projection menolak setiap path yang memiliki ancestor/component `DISABLED*` | repository matrix tests |
| KV-07 | Resolved | KeyViewer hanya memanen TextureOverride 8-hex; ShaderOverride 16-hex masuk typed conflict scope | harvester negative/positive tests; conflict tests |
| KV-08 | Resolved | Hash input didedupe, threshold dinaikkan, sentinel wajib unik dan collision ambigu dibuang | matcher duplicate/shared/ambiguous fixtures |
| KV-09 | Resolved | Artifact dibuat di staging unik, file/directory swap recoverable, dead fallback dihapus, error cleanup/write dipropagasi | atomic/generator failure tests dan caller warning propagation |
| SW-01 | Resolved | Rust/TypeScript memakai semantics case-insensitive `DISABLED*`; rename tetap canonical `DISABLED ` | normalizer/path-key suites |
| SW-02 | Resolved | Default menjadi `Ctrl+F6`/`Ctrl+F8`; UI memeriksa F6, F8, dan reload key hasil discovery tanpa memigrasi config tersimpan | Rust hotkey suite dan `HotkeyTab.test.ts` |
| SW-03 | Resolved | Rollback yang tidak lengkap menghasilkan warning; collection pipeline menjalankan full disk reconcile dan menyimpan recovery warning di progress state | runtime mutation rollback tests; batch reconcile source audit |
| SW-04 | Resolved | Workspace/app mutation menyebut disk state applied + reload required; in-game cycle baru melaporkan reload setelah key-send sukses | workspace switch UI test dan hotkey reload flow tests |
| INI-06 | Resolved | Archive classifier menerima Include/Key/CommandList/ShaderRegex selain override/resource | archive classify fixtures |
| KV-10 | Resolved | Conflict report membawa kind, provenance, namespace, condition, priority, `match_first_index`, shader stage, serta certainty potential/definite; indeks yang pasti terpisah tidak digabung | conflict evidence, disjoint-index, dan ShaderFixes-only tests |

Temuan KV-05 adalah **runtime-test** untuk menetapkan satu bentuk path canonical; mixed resolution-nya terverifikasi dari generator/caller dan source resolver upstream. Urutan `Present`/TextureOverride dan flicker threshold 1.5 detik juga harus diuji in-game pada setiap package, bukan disimpulkan hanya dari unit test string.

## 8. Roadmap yang disarankan

### Fase A — samakan kontrak dengan runtime

1. Buat `GameRuntimeProfile`: lokasi `d3dx.ini`, namespace/API text, built-in reserved hotkeys, reload binding grammar, dan root `Mods` per package.
2. Buat satu `EffectiveEnabledResolver` yang mempertimbangkan semua ancestor dan pola exclude runtime; gunakan untuk KeyViewer, conflicts, collection preview, dan switch status.
3. Ganti discovery INI dengan walker rekursif yang mengikuti policy `include_recursive/exclude_recursive`, deterministik, tidak mengikuti symlink keluar root, dan melaporkan error.
4. Pisahkan parser binding 3Dmigoto dari parser global-hotkey EMMM. Parse `[Hunting] reload_fixes`, token spasi, `no_*`, `VK_*`, multiple main/controller tokens; replay hanya bila game terverifikasi fokus.

### Fase B — reader/writer benar-benar lossless dan recoverable

1. Simpan source bytes metadata: encoding, BOM, per-line terminator/final newline, content hash, dan file identity/mtime.
2. Modelkan ordered tokens/lines; structured indices adalah view di atas token, bukan representasi pengganti.
3. Dukung variable declaration qualifiers dan repeated `key/back` sambil mempertahankan urutan/case.
4. Sebelum save, validasi semua updates, re-read dan bandingkan hash, lalu write ke unique temp pada directory yang sama.
5. Gunakan replace atomic Windows yang benar atau recovery transaction: fsync/flush, replace, auto-restore backup jika commit gagal, dan per-file lock.

### Fase C — KeyViewer portable dan deterministik

1. Migrasikan renderer ke API text resmi per profile (`GIMIv8\PrintText`, adapter WWMI, dan capability check/fallback).
2. Tetapkan satu base path relatif dan uji dari lokasi aktual `.emmm_data/KeyViewer.ini`.
3. Gunakan typed hashes (`Shader64`, `Resource32`) dan dedupe per object/skin/provenance sebelum scoring.
4. Kalibrasi matcher dengan fixture nyata; paksa sentinel unik atau selesaikan ambiguity eksplisit. Jangan menghasilkan override untuk match tanpa sentinel.
5. Buat artifact generation transactional: staging directory, validate INI/resources, atomic swap, cleanup stale, dan propagate semua error.

### Fase D — switch lifecycle dan UX

1. Reserved-hotkey validator membandingkan binding EMMM dengan profile package dan `[Hunting]/[Key*]` aktual; ubah default F6/F8.
2. Bedakan status `DiskApplied`, `ArtifactsRefreshed`, dan `RuntimeReloaded/ReloadRequired`.
3. Bila rollback gagal, paksa full reconcile dan tampilkan recovery action, bukan hanya warning log.
4. Tambahkan integration test rename object ancestor: runtime exclude, DB effective state, KeyViewer harvest, conflicts, collection signature, dan F10 behavior harus konvergen.

## 9. Acceptance test minimum

- Fixture nested mod dengan UTF-8 BOM, Shift-JIS, CRLF/LF campuran, final/no-final newline, repeated key/back, qualified variables, flow control, dan non-ASCII section.
- Golden round-trip: no-op save menghasilkan bytes identik; targeted save hanya mengubah token yang diminta.
- Stale-write test: file diubah eksternal sesudah read harus menghasilkan conflict, bukan overwrite.
- Failure injection: temp write, replace, backup, dan rollback failure selalu meninggalkan original atau backup yang dapat dipulihkan.
- Package matrix GIMI/WWMI/SRMI/ZZMI: lokasi config, namespace text, F6/F8/F10/F12, relative resource path, dan reload grammar.
- Effective-enabled matrix untuk disabled prefix pada setiap ancestor dan variasi legacy/case.
- Matcher fixtures dari asset repositories: shared texture, duplicate skin hashes, shader64/resource32, zero sentinel, dan ambiguous sentinel.
- In-game smoke: toggle mod/object/preset, F10, camera pan, character swap, safe mode, status fallback, dan overlay off tidak meninggalkan notification stale.

### 9.1 Snapshot repository yang diperiksa

Commit ini dicatat agar klaim dapat direproduksi dan diperbarui tanpa mengandalkan “latest” yang bergerak:

| Repository | Branch | Commit snapshot |
|---|---|---|
| `bo3b/3Dmigoto` | `master` | `4ce5f2f72b2777223d2e809dcdafec514ac98295` |
| `rayshire/3dmigoto` | `main` | `7a1255b45cac2133c4adc4c697cc7241263fa722` |
| `SilentNightSound/GIMI-Package` | `main` | `a88633c677766b81290d0de9f91a879f862c2bc0` |
| `SilentNightSound/GI-Model-Importer` | `main` | `4232c2679193cad7f15898a20517798560d38153` |
| `SilentNightSound/GI-Model-Importer-Assets` | `main` | `2039d16d4b64696098ba53cd69888ce967397be9` |
| `SilentNightSound/SR-Model-Importer` | `main` | `ecb4a4708bceb5e134bc571a9bbc43fd10c21363` |
| `SilentNightSound/SR-Model-Importer-Assets` | `main` | `eff6cdf613cb3b07cc241ca3f287abe3b1496b71` |
| `SpectrumQT/WWMI-Package` | `main` | `647462518c1916e04dea1de9048f152326960795` |
| `leotorrez/ZZ-Model-Importer` | `main` | `4fb37188b8f1b509e0bfcef877b236a8ade6e4d4` |
| `SpectrumQT/XXMI-Launcher` | `main` | `d56786b8dacb00c35204bff45ff5b8b83bd8962a` |

Catatan ecosystem freshness: XXMI Launcher saat ini menunjuk package SRMI/ZZMI modern yang berbeda dari repository standalone lama pada daftar awal (`SpectrumQT/SRMI-Package` dan `leotorrez/ZZMI-Package`). Repository lama tetap berguna untuk guides/tools, tetapi profile runtime produksi harus mengikuti package yang benar-benar dipasang launcher.

## 10. Referensi primer

- [bo3b/3Dmigoto](https://github.com/bo3b/3Dmigoto) dan [Dependencies/d3dx.ini](https://github.com/bo3b/3Dmigoto/blob/master/Dependencies/d3dx.ini)
- [GIMI-Package](https://github.com/SilentNightSound/GIMI-Package), [d3dx.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/d3dx.ini), [API.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/API.ini), [help.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/help.ini), dan [KeyBindings.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/KeyBindings.ini)
- [WWMI-Package](https://github.com/SpectrumQT/WWMI-Package) dan [WWMI KeyBindings.ini](https://raw.githubusercontent.com/SpectrumQT/WWMI-Package/main/WWMI/Core/WWMI/KeyBindings.ini)
- [GI-Model-Importer](https://github.com/SilentNightSound/GI-Model-Importer), [GI assets](https://github.com/SilentNightSound/GI-Model-Importer-Assets), [SR importer](https://github.com/SilentNightSound/SR-Model-Importer), dan [SR assets](https://github.com/SilentNightSound/SR-Model-Importer-Assets)
- [ZZ-Model-Importer](https://github.com/leotorrez/ZZ-Model-Importer), [XXMI-Launcher](https://github.com/SpectrumQT/XXMI-Launcher), dan [rayshire mirror](https://github.com/rayshire/3dmigoto)
- [Leotorrez INI docs](https://leotorrez.github.io/modding/docs/), [Hunting](https://leotorrez.github.io/modding/guides/hunting), [Textures 101](https://leotorrez.github.io/modding/guides/textures-101), dan [Shaders 101](https://leotorrez.github.io/modding/guides/shaders-101)
