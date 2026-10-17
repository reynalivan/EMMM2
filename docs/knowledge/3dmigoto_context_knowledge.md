# 3DMigoto Context Knowledge for EMMM2NEW

> Knowledge base and technical guide as of 2026-08-11. Use it to understand the runtime, write/diagnose mods, and integrate EMMM with XXMI packages. Gap status and remediation results are in [`3dmigoto_gap_status.md`](./3dmigoto_gap_status.md).

## 1. Purpose and source boundaries

EMMM focuses on four interconnected subsystems:

1. INI Reader/Formatter: reads mod files without altering their original form and extracts editable variables/key bindings.
2. INI Writer: changes selected lines with a backup and safe replacement.
3. KeyViewer: harvests hashes/keybinds from active mods, matches them against MasterDB, and then creates a 3DMigoto overlay.
4. Mod switching: enables/disables folders through the `DISABLED ` convention, reconciles disk/DB state, updates overlay artifacts, and reloads runtime configuration when safe.

Source trust hierarchy:

- Primary: [bo3b/3Dmigoto](https://github.com/bo3b/3Dmigoto) source code, XXMI package configuration files, and local EMMM code.
- Supporting: READMEs, guides, releases, and asset databases maintained by the related project owners.
- Tutorial: [leotorrez modding docs](https://leotorrez.github.io/modding/docs/); useful for workflows, but the pages themselves state that their syntax is GIMI/XXMI-oriented and not all of it applies to upstream 3Dmigoto.
- `https://www.3dmigoto.com`: scraped results are not used as a knowledge source. The official bo3b README warns that this domain is phishing and is not owned by the project; a snapshot previously taken by the research worker contained only an overview/marketing and does not override that warning. The official reference remains [bo3b on GitHub](https://github.com/bo3b/3Dmigoto).
- Context7: a search for `3Dmigoto` returned “No libraries found”; Context7 is not a suitable source for this project. GitHub source is the primary fallback.

Claude conversation reviewed: `E:/Downloads/3dmigoto_EMMM2NEW.md`. The last deep-research workflow in that conversation failed entirely because of a session limit, so its earlier conclusions were re-verified here rather than accepted at face value.

## 2. Repository ecosystem map

| Repository                                                                                  | Key contents                                                                                                             | Role in the system                                                                       |
| ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| [bo3b/3Dmigoto](https://github.com/bo3b/3Dmigoto)                                           | DirectX 11 wrapper/hook, INI parser, command-list engine, hunting/frame analysis, loader, example `Dependencies/d3dx.ini` | Official upstream and source of runtime semantics                                       |
| [rayshire/3dmigoto](https://github.com/rayshire/3dmigoto)                                   | Fork of `SilentNightSound/GI-Model-Importer`; root contains Guides, Tools, and GIMI ZIPs                                  | Not a direct fork of bo3b upstream; an older GIMI mirror/variant                         |
| [SilentNightSound/GIMI-Package](https://github.com/SilentNightSound/GIMI-Package)           | `GIMI/d3dx.ini`, `Core/GIMI`, shader/font/API/notification                                                               | Modern GIMI package for XXMI; relevant to the current runtime contract                   |
| [SilentNightSound/GI-Model-Importer](https://github.com/SilentNightSound/GI-Model-Importer) | Guides, Blender tools/scripts, dev/play ZIPs                                                                            | Older standalone GIMI; README says it is deprecated and points users to XXMI              |
| [GI-Model-Importer-Assets](https://github.com/SilentNightSound/GI-Model-Importer-Assets)    | Player/NPC/enemy/weapon/skill data, buffers/textures, `hash.json`                                                        | Source of GI hash/model facts for MasterDB/importer                                      |
| [SR-Model-Importer](https://github.com/SilentNightSound/SR-Model-Importer)                  | Port of importer/tools/guides for HSR                                                                                    | HSR tooling/export-import based on GIMI patterns                                        |
| [SR-Model-Importer-Assets](https://github.com/SilentNightSound/SR-Model-Importer-Assets)    | Player/weapon/skill/enemy assets and `hash.json`                                                                         | Source of HSR hash/model facts                                                          |
| [SpectrumQT/WWMI-Package](https://github.com/SpectrumQT/WWMI-Package)                       | `WWMI/d3dx.ini`, `Core/WWMI`, custom shader examples                                                                     | XXMI package for Wuthering Waves; its namespace/API must not be assumed to match GIMI    |
| [leotorrez/ZZ-Model-Importer](https://github.com/leotorrez/ZZ-Model-Importer)               | ZZMI ZIP, Tools, XXMI instructions                                                                                        | ZZZ variant; F10 hot-load and F6 package toggle                                         |
| [SpectrumQT/XXMI-Launcher](https://github.com/SpectrumQT/XXMI-Launcher)                     | Python app, core/gui, locale, themes, installer/updater                                                                  | Installation/launch/config orchestrator; the runtime mod remains a per-game 3Dmigoto package |

In simple terms: upstream 3Dmigoto provides interception and the INI language; GIMI/SRMI/WWMI/ZZMI packages adapt the runtime for each game; asset repositories provide hashes and model data; Blender tools produce resources/INI files; XXMI Launcher installs and runs those packages; EMMM manages mod folders and runtime artifacts on top of the `Mods` layout.

### 2.1 File-level tour: bo3b/3Dmigoto

The upstream root shows the following responsibility boundaries:

- `DirectX11/`: device/context/resource wrappers and the primary DX11 interception points. This is where overrides can react to active draws/resources.
- `DirectXGI/`: factory, swap-chain, and present path. The overlay and `[Present]` work depend on this lifecycle.
- `DirectX9/` and `DirectX10/`: cross-version compatibility components; XXMI fork features cannot all be inferred from the DX11 label alone.
- `Injector/`: loader/injection flow. An XXMI package may place the loader/runtime separately from the game executable.
- `HLSLDecompiler/`, `BinaryDecompiler/`, and `D3D_Shaders/`: hunting, disassembly/decompilation, and shader tooling.
- `D3DCompiler*`: wrapper/support for several shader compiler versions.
- `Dependencies/`: runtime templates, including `d3dx.ini`, a practical source for key grammar, includes, hunting, rendering, resources, and the loader.
- `NVAPI/`: legacy NVIDIA/stereoscopic integration; not part of EMMM core, but it explains the project's origins.
- `ini_parser_lite.cpp`, `vkeys.h`, and DirectX11 command-list sources are references when documentation and package behavior differ.
- `StereovisionHacks.sln` is the upstream build solution. This repository is developer source, not a ready-to-use end-user distribution.

The README or `d3dx.ini` template describes the expected contract, but parser/executor source remains the final authority for edge cases such as duplicate properties, path resolution, namespaces, and pre/post ordering.

### 2.2 File-level tour: XXMI runtime packages

#### GIMI-Package

Verified core layout:

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

- `d3dx.ini` selects the target/loader, includes `Core\GIMI\main.ini`, and recursively scans `Mods`.
- `main.ini` is the GIMI composition root: the core subsystems, text renderer, notifications, and package variables meet there.
- `API.ini` is the public contract for features such as `CommandList\GIMIv8\PrintText`.
- `KeyBindings.ini` defines F6/F10/F12 and package variables.
- `help.ini` provides a bridge for older mods, but its comments mark it as deprecated.
- `d3dx_patch.ini` is a package patch/config extension, not an ordinary user mod.
- Fonts/Shaders/Notifications are renderer dependencies; hardcoding one file without understanding the dependency graph risks breakage when the package changes.

#### WWMI-Package

Verified core layout:

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

The differences are not cosmetic. The `WWMIv1` namespace, composition root, utilities, compatibility mode, and command-list API can differ from `GIMIv8`. `GameType -> RuntimeProfile` must select capabilities, not merely replace the game-name string.

#### SRMI and ZZMI

- Standalone SRMI/ZZMI repositories contain development/play ZIPs and tools, not a modern package source tree as complete as GIMI-Package/WWMI-Package.
- The SRMI README says many files were ported from GIMI and may still contain Genshin references; do not assume compatibility just because the structure is similar.
- ZZMI directs installation through XXMI Launcher and uses F6 to toggle dependent mods, F10 to reload/save, and F12 for the guide.
- SRMI/ZZMI profiles should be derived from the actual package installation selected by the user, not only from old repository ZIPs.

### 2.3 File-level tour: importer tools

`GI-Model-Importer/Tools` provides an overview of the authoring pipeline:

- `blender_3dmigoto_gimi.py`: Blender import/export add-on with 3dmigoto custom properties.
- `genshin_3dmigoto_collect.py`: legacy collect/dump tooling.
- `genshin_3dmigoto_generate.py`: generates buffers/resources and formats exported INI files.
- `genshin_merge_mods.py`: merges mods; the result can contain nested INI files and a structure more complex than a single simple mod.
- `genshin_animation_creator.py`, damage merge, outline, transparency, and color scripts show that mod INI files can contain rich command lists/variables.
- Blender scripts for bone deletion, vertex-group remapping/merging/filling, unused-group cleanup, and custom-property transfer preserve model metadata, but are outside the EMMM INI editor's responsibility.

Valid mod files can be created by people, generators, or merge tools. The EMMM Reader/Writer must not reformat an entire file based on a single example style.

#### 2.3.1 `merged.ini` and subvariant ownership

`genshin_merge_mods.py` builds one `merged.ini` orchestrator: cycle variables such as `$swapvar` select a branch/CommandList, while resources and files belonging to several source variants remain in descendant folders. Source-variant INI files are usually disabled so 3DMigoto does not execute the orchestrator and source files at the same time.

Consequences for Storage Optimizer:

- the merged output root and all descendants form one logical mod unit, not a set of duplicate mods;
- `DISABLED` status on a child INI is part of orchestration and does not prove that the child asset can be deleted safely;
- `merged.ini` is only a hint; the primary ownership boundary remains the terminal root identified by the classifier/walker;
- the same 3DMigoto target hash can indicate a subvariant, recolor, or runtime conflict. It does not prove byte identity;
- partial content hashes are only for prefiltering. Keep/Delete/Hardlink actions require a path-aware manifest of all regular files and matching full BLAKE3 hashes;
- TextureOverride sections accept 8-hex resource hashes, while ShaderOverride/replacement uses 16-hex shader hashes. These typed targets help with explanations/conflict review but do not authorize destructive actions.

The shared decoder currently supports UTF-8 (with or without a BOM) and Shift-JIS fallback. UTF-16 LE/BE is not yet a supported capability; mods using it must be rejected/flagged until both the reader and lossless writer support it.

Branch semantics depend on the order in which 3DMigoto processes command sections. Flattening child variants or transitively combining relationships can therefore change behavior even when some textures/buffers happen to be identical.

### 2.4 File-level tour: asset repositories

GI assets divide data into `PlayerCharacterData`, `NPCData`, `EnemyData`, `WeaponData`, `SkillData`, and `MiscellaneousData`. SR assets use `PlayerCharacterData`, `WeaponData`, `SkillObjData`, and `EnemyData`.

An object folder can contain models/buffers/textures and `hash.json`. The `hash.json` structure is an array of components, not one flat object:

| Field                    | Practical meaning                                                    | Characteristics             |
| ------------------------ | -------------------------------------------------------------------- | ---------------------------- |
| `component_name`         | Part name such as Hair/Body/Face, or empty for the main component    | Not always unique/global     |
| `root_vs`                | Root vertex shader                                                   | Usually 16 hex; may be empty |
| `draw_vb`                | Hash buffer pengenal draw/model                                      | 8 hex                        |
| `position_vb`            | Vertex positions                                                     | 8 hex                        |
| `blend_vb`               | Weight/bone blend                                                    | 8 hex                        |
| `texcoord_vb`            | UV/tangent/normal stream sesuai game                                 | 8 hex                        |
| `ib`                     | Index buffer                                                         | 8 hex                        |
| `object_indexes`         | Component first-index/slice                                           | Integer array                |
| `object_classifications` | Per-index part label                                                  | Array aligned with indexes   |
| `texture_hashes`         | Texture type, extension, and hash per classification                 | Nested array; hash 8 hex     |

Albedo has a main component with a 16-hex `root_vs` and a separate `Face` component with some empty fields. Blade in SR has Hair, Head, and Body with several classifications. The MasterDB loader must preserve game/object/component/skin/hash-type provenance; flattening without types removes information important to the matcher.

### 2.5 File-level tour: XXMI Launcher

The launcher root contains `src/xxmi_launcher`, `Locale`, `Themes/Default`, `public-media`, and `requirements.txt`. Its responsibilities are to:

- install the correct model-importer package;
- store launch configuration and paths;
- update the launcher and package instances;
- verify download/library authenticity according to the README;
- start the game/runtime with a layout that may differ from the game executable folder.

EMMM should not assume `game_exe.parent()` is the package root. `mod_path`, `loader_exe`, and discovery of the actual XXMI installation are stronger sources for the runtime location.

### 2.6 Source status matrix

| Source                       | Use                                         | Caveat                                                              |
| ---------------------------- | ------------------------------------------ | ------------------------------------------------------------------- |
| bo3b master/source           | Upstream semantics and configuration templates | XXMI forks include their own patches/extensions                  |
| GIMI-Package/WWMI-Package    | Modern package contract                    | Changes with package updates; version detection is needed           |
| Standalone GI/SR/ZZ importer | Guides and authoring tools                  | Some are deprecated/outdated; not always a modern runtime            |
| Asset repositories           | Per-commit ground truth for hashes/models  | Hashes may become stale after game updates; commit/version metadata is needed |
| Leotorrez docs               | Modder workflows and syntax examples       | GIMI-oriented and includes simplifications                          |
| Claude conversation          | Audit hypotheses and direction             | Deep research failed; all findings must be re-verified               |

## 3. 3Dmigoto runtime mental model

3Dmigoto wraps/hooks the DirectX path, observes resources and draw calls, and then runs command lists compiled from INI on events such as shader/resource matches and `Present`. This is not a generic INI parser: some sections are ordinary configuration, while `TextureOverride*`, `ShaderOverride*`, `CommandList*`, `CustomShader*`, and some global sections form a runtime program.

Basic flow:

```text
d3dx.ini -> [Include] -> include files/Mods tree -> parse + namespace
          -> Texture/ShaderOverride matches a draw/resource
          -> pre/post command lists change state/resource/draw
          -> Present runs per-frame/overlay work
          -> F10 reload_fixes + reload_config loads runtime changes
```

Key configuration facts from [upstream d3dx.ini](https://github.com/bo3b/3Dmigoto/blob/master/Dependencies/d3dx.ini) and [GIMI d3dx.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/d3dx.ini):

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

Implications for EMMM:

- Disable is a filesystem rule evaluated during include traversal. Renaming a folder mutates the source of truth; the DB is only a projection.
- `DISABLED*` is broader than `DISABLED `: the runtime can exclude any name beginning with that word.
- F10 is required to hot-load new configuration/INI. Not all settings can be reloaded without a restart.
- 3Dmigoto bindings are space-delimited, support positive/negative modifiers (`ctrl`, `no_shift`, `no_modifiers`) and `VK_*` names; this differs from EMMM's global-hotkey UI format, which uses `Shift+F6`.
- GIMI and WWMI use F6 to toggle all package mods; F8 is used for frame analysis. EMMM's default F6/F8 bindings conflict semantically.

### 3.1 Runtime section taxonomy

| Section/prefix       | Role                                                                      | Parser/editor implications                                                          |
| -------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `[Include]`          | Selects additional INI files/trees and exclusions                         | Defines the active file universe; must inform discovery policy                       |
| `[Loader]`           | Target, loader, module, elevation/launch behavior                         | Helps discover the package root but is unsafe to edit as an ordinary mod            |
| `[Hunting]`          | Hunting state, marking, cycle keys, F8/F10, dump options                  | 3dmigoto-specific binding grammar, not `[Key*]`                                     |
| `[Rendering]`        | Hash mode, shader/cache directories, resource tracking                    | Changes may require a restart or cache invalidation                                  |
| `[Constants]`        | Global/persistent variables and initialization commands                  | Declaration qualifiers and ordering must be preserved                                |
| `[Key*]`             | Hold/toggle/cycle, repeated `key`, `back`, condition, command list        | A one-key struct is insufficient; multiple bindings are valid                       |
| `[TextureOverride*]` | Matches resource/buffer hashes and runs commands/draw/resource replacement | Hash is usually 8-hex; section is executable and order-sensitive                    |
| `[ShaderOverride*]`  | Matches shaders and runs overrides/filters/commands                       | Hash is usually 16-hex; do not force it into the resource32 type                     |
| `[Resource*]`        | Buffer/texture/file/ref/data declarations                                 | Paths, formats, bind flags, and namespaces matter                                    |
| `[CommandList*]`     | Ordered executable commands and flow control                              | Duplicate commands form a sequence, not a conflicting key                            |
| `[CustomShader*]`    | Custom shader/render-state pipeline                                       | May reference external shader files and resources                                    |
| `[Present]`          | Per-frame driver after/around present                                     | Multiple included files may contribute; ordering and pre/post need runtime testing   |

### 3.2 Grammar binding 3dmigoto

Examples of common/valid values:

```ini
key = F6
key = no_modifiers F6
key = CTRL F12
reload_fixes = no_modifiers VK_F10
wipe_user_config = ctrl alt no_shift VK_F10
next_marking_mode = no_modifiers VK_DECIMAL VK_NUMPAD0
key = XB_X
```

The target parser should produce a semantic model, for example:

```text
required modifiers   = {Ctrl, Alt}
forbidden modifiers  = {Shift}
main chord keys      = {F10}
controller binding   = optional XB_*
  source spelling      = preserve for round-trip
```

`no_modifiers` is not a key to press; it is shorthand for a constraint. `VK_` is an optional prefix for virtual-key names. Some hunting bindings form chords with more than one non-modifier key, so a function that treats "all tokens except the last as modifiers" is still too limited.

### 3.3 Reload and persistence lifecycle

- `reload_fixes` rereads shader fixes; `reload_config` rereads configuration/INI. Packages commonly bind both to F10.
- Persistent variables can be written to `d3dx_user.ini`; `wipe_user_config` clears that state and then reloads.
- Renaming a folder changes the result of include traversal on the next reload; it does not guarantee that an already-running runtime immediately stops using the old command list.
- Reloading can be expensive when there are many shaders/INI files. Modern GIMI has delay/cache options; EMMM should debounce and show progress instead of spamming F10.
- Some `d3dx.ini` settings are not reloadable and still require a restart. EMMM should say “reload requested” instead of promising that all runtime state has changed.

### 3.4 When a mod is effectively enabled

A child mod is effectively enabled only if all of these conditions are true:

```text
child folder does not match an exclusion
AND every ancestor up to the Mods root does not match an exclusion
AND the INI file is included by the include policy
AND the runtime has loaded the latest configuration
AND package conditions/variables allow the command list
```

`mods.status = 1` only reflects the child's prefix in the DB projection. It does not prove effective activation. This is why `EffectiveEnabledResolver` should be a shared primitive, not extra logic specific to KeyViewer.

## 4. INI semantics to preserve

### 4.1 Structure and namespaces

- Section/property names are case-insensitive.
- `namespace = ...` can change a file's namespace. Cross-namespace references use forms such as `Resource\GIMIv8\Text`.
- Included files can have namespaces based on file/path; global sections such as `[Present]` and `[Constants]` may contribute to a global/combined runtime, so collisions and execution order matter.
- Command lists allow ordered commands, repeated commands, `if`/`else if`/`else`/`endif` flow control, and `pre`/`post` phases. A plain `Map<String,String>` is not sufficient for lossless representation.
- Upstream supports multiple `key =` entries in one `[Key*]` and `back =` for reverse cycling.
- Whole-line `; ...` comments are safe; the Leotorrez tutorial warns that inline comments can cause compilation errors even if a syntax highlighter accepts them.

### 4.2 Hashes and resources

- Shader hashes are usually 16 hex/64-bit, e.g. `root_vs = 653c63ba4a73ca8b`.
- Buffer/texture hashes are usually 8 hex/32-bit, e.g. `ib = 0d7dc936` or texture `9bd05da3`.
- The [Albedo `hash.json`](https://raw.githubusercontent.com/SilentNightSound/GI-Model-Importer-Assets/main/PlayerCharacterData/Albedo/hash.json) example stores `root_vs`, draw/position/blend/texcoord VB, IB, index/classification, and per-component texture hashes.
- `TextureOverride` does not mean “texture file only”; importers also often match IB/VB through resource override mechanisms.
- A resource `filename` should be treated as relative to the context of the INI/package file that contains it; test the path contract with a file placed at its actual production location.

The upstream resolver first tries a path relative to the namespace/config file, then falls back to a path relative to the 3dmigoto root. Two path forms may both appear to work while using different resolution branches. EMMM should choose one explicit contract and not depend on implicit fallback behavior.

### 4.2.1 A hash alone is not enough to judge a match

`TextureOverride*` can narrow a match with `match_first_index`, first vertex/instance, count, format/dimension/resource description, condition, and `match_priority`. Two sections with the same hash are not necessarily in conflict if their draw contexts differ. Conversely, a shared hash used without context can activate KeyViewer for the wrong object.

`ShaderOverride*` and `ShaderRegex*` can also enable `checktextureoverride` on specific slots (`ps-t0`, `vb0`, `ib`). Therefore, “the hash appears in the INI” does not always mean the runtime evaluates that hash on every draw.

Stronger evidence contract for MasterDB/KeyViewer:

```text
game + package version
object + component + skin
hash type/width
override section type
draw/resource context
source file/namespace
first-index or other constraints
confidence/provenance
```

### 4.3 Text overlay in modern GIMI

[GIMI API.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/API.ini) defines the supported API:

```ini
Resource\GIMIv8\Text = ref ResourceMyText
Resource\GIMIv8\TextParams = ref ResourceMyTextParams
run = CommandList\GIMIv8\PrintText
```

[GIMI help.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/help.ini) is a deprecated compatibility layer. `CustomShaderFormatText` is only a dummy; actual rendering forwards the resource to `CommandList\GIMIv8\PrintText`. Because WWMI uses the `WWMIv1` namespace, EMMM needs an adapter/capability per game/package instead of hardcoding GIMI.

## 5. Hunting, texture, and shader workflows

Sources: [Hunting](https://leotorrez.github.io/modding/guides/hunting), [Textures 101](https://leotorrez.github.io/modding/guides/textures-101), and [Shaders 101](https://leotorrez.github.io/modding/guides/shaders-101).

1. Enable hunting before launch; enable shader dumps for shader editing.
2. Disable mods that could interfere with the target. F6 toggles the package, but some shaders may remain active; renaming an ancestor with `DISABLED` is stronger.
3. Numpad 0 mengaktifkan hunting; cycle VS/PS/IB/VB/CS lalu mark/copy hash.
4. F8 creates a frame analysis. Global dumps can be very large; target resources through overrides/dump commands when possible.
5. `gui_collect` converts dumps into IB/VB/texture plus `hash.json`, then the Blender importer/exporter creates mod resources and INI.
6. Texture replacements must preserve the appropriate format/color space. Shader replacements use 16-hex hashes and replacement files/resource slots, and can receive IniParams/time from the INI.
7. Shader hashes change after game updates more often than some model/resource hashes; the database and matcher need versioning/provenance.

## 6. Current EMMM implementation

### 6.1 INI Reader/Formatter

- [`document.rs`](../src-tauri/src/services/ini/document.rs) discovers INI files recursively and deterministically, excludes `desktop.ini` and `DISABLED*` subtrees, and does not follow symlinks.
- [`encoding.rs`](../src-tauri/src/services/ini/encoding.rs) preserves the original encoding, BOM, per-line terminators (`LF`, `CRLF`, `CR`, or none), and final-newline state.
- The structured view is an index over raw lines. It recognizes `global`, `persist`, and `local` qualifiers, preserves section spelling, and stores repeated `key`/`back` entries in source order.
- Lossy input can be read as a raw fallback, but must not be saved because the original bytes cannot be safely reconstructed.

### 6.2 INI Writer

- [`write.rs`](../src-tauri/src/services/ini/write.rs) validates all updates before side effects, then compares the BLAKE3 source fingerprint with the latest bytes on disk.
- The writer preserves the original encoding/BOM/terminators and rejects characters that cannot be represented in Shift-JIS.
- Commit uses a unique sibling temp file, recovery rename, automatic restore on replacement failure, and three-generation backup rotation.
- The write command runs under `OperationLock`; an external edit after reading produces a stale-write error instead of an overwrite.

### 6.3 KeyViewer

- [`post_apply.rs`](../src-tauri/src/services/app/post_apply.rs) uses effectively enabled paths, harvests nested INI files in one pass, matches objects, and then builds the artifact in staging before swapping the directory.
- [`harvester.rs`](../src-tauri/src/services/keyviewer/harvester.rs) only feeds 8-hex resource hashes from `TextureOverride*` into the matcher. The conflict scanner handles 16-hex shader hashes as a different type.
- [`matcher.rs`](../src-tauri/src/services/keyviewer/matcher.rs) deduplicates evidence, uses a conservative threshold, and only produces an artifact when the sentinel is unique across all object results.
- [`generator/ini.rs`](../src-tauri/src/services/keyviewer/generator/ini.rs) uses each package's official text API: `GIMIv8`, `SRMIv1`, `WWMIv1`, or `ZZMIv1`. EFMI is explicitly rejected until its official API is verified.
- All resource paths are written relative to `.emmm_data/KeyViewer.ini` through one stable directory: `generations/status/...` and `generations/keybinds/active/...`. No versioned generation subfolders are read by 3DMigoto.

### 6.4 Switch mod

- [`core_ops/toggle.rs`](../src-tauri/src/services/mods/core_ops/toggle.rs) and [`runtime_mutation_engine.rs`](../src-tauri/src/services/runtime_mutation_engine.rs) perform filesystem renames with collision checks, locking/suppression, batch plans, and rollback.
- [`workspace_switch_service.rs`](../src-tauri/src/services/workspace_switch_service.rs) separates object-root switching from mod switching and runs scoped disk reconciliation.
- If rollback is incomplete, the collection pipeline runs a full disk reconciliation and stores a recovery warning in progress state.
- UI switching reports **disk applied, reload required** and shows the discovered reload key. In-game preset cycling reports a reload only after key-send succeeds.
- EMMM defaults to `Ctrl+F6` and `Ctrl+F8`; F6/F8 without modifiers remain owned by the package/3DMigoto.

### 6.5 Call flow Reader -> Editor -> Writer

```text
mod path
  -> list_ini_files (recursive, sorted, skip desktop.ini/DISABLED*/symlink)
  -> read_ini_document
       -> metadata size guard 2 MiB
       -> read bytes
        -> detect UTF-8/BOM, Shift-JIS, or lossy fallback
        -> split raw_lines while preserving each line terminator
        -> parse qualified variables and repeated [Key*] key/back
        -> calculate source fingerprint
   -> frontend creates line_updates
  -> save_ini_with_updates
        -> validate all updates and reject RawFallback
        -> re-read target and compare fingerprint
       -> patch cached raw_lines by index
        -> encode using the original encoding/BOM/terminators
       -> rotate backup + write unique temp + sync
        -> recoverable replace; auto-restore if commit fails
```

Invariants that must be preserved:

- raw lines remain the source for rerendering, not a serializer over structured fields;
- saving is rejected if decoding is not trusted or the source has changed;
- a targeted edit does not alter encoding, terminators, BOM, or other lines;
- all updates are validated before a backup/temp file is created;
- the original target or a clear recovery option is always available if replacement fails.

### 6.6 Call flow mutation -> reconcile -> overlay

```text
UI/workspace/collection/hotkey mutation
  -> operation lock + watcher suppression
   -> rename folder(s) on disk
  -> scoped disk reconcile
       -> scan/classify disk
       -> settle object/mod rows + runtime projection
        -> collection dirty-state if needed
       -> trigger_overlay_refresh_for_game
            -> rebuild projection
            -> query enabled child rows
            -> conflict scan
            -> hash/keybind harvest
            -> DB object/hash match
            -> KeyViewer.ini + text + status
   -> caller reports the reload key or the hotkey path replays the key
```

The flow has these mature design elements:

- disk is treated as the source of truth;
- object-root switching is separated from mod-level switching;
- operation lock and watcher suppression reduce race conditions/feedback loops;
- batch mutations are planned before renaming and have rollback + full-reconciliation recovery;
- reconciliation becomes the single DB writer after the filesystem changes;
- the effectively-enabled query is used after the projection settles;
- runtime artifacts are built in staging and swapped only after validation.

Boundaries that still need to be understood:

- successful rename/reconciliation/artifact generation does not mean the game has reloaded;
- UI-driven mutations may only show `ReloadRequired`, because game focus is not guaranteed;
- an in-game hotkey may show `RuntimeReloaded` only after input replay succeeds;
- an artifact error after a disk mutation is degraded completion, not grounds to claim that the disk was rolled back.

### 6.7 KeyViewer call flow in detail

```text
get_enabled_mods_paths
   -> filter status=enabled + no DISABLED* component
   -> for each mod: list nested INI
   -> decode once
  -> harvest TextureOverride resource hash 8-hex
   -> parse all repeated [Key*] entries
  -> occurrence_counts + hash_to_mod_path
  -> get_kv_matching_objects
  -> dedupe skin/code hashes
  -> match_objects
       -> reverse hash -> object-name set
       -> intersection + score
       -> threshold filter
       -> wajibkan sentinel unik; drop ambiguity
   -> render API text based on GameType
   -> map sentinels back to source mod keybinds
   -> stage keybind text + status + KeyViewer.ini
   -> atomic directory/file replacement
```

Diagnostic rules:

- one base hash is not enough to accept a match;
- duplicate evidence must not raise the score;
- a shared sentinel must not produce two TextureOverride artifacts;
- a 16-hex ShaderOverride must not be forced into the resource matcher;
- create/write/replace/cleanup failures must be visible as errors or degraded warnings;
- EFMI must be treated as unsupported, not silently assigned another package's namespace.

### 6.8 Hotkey ownership

Three hotkey domains must be kept separate:

1. EMMM's global OS hotkey, registered by Tauri and entered by the user as `Shift+F6`.
2. A key section in a mod/package, processed by 3dmigoto while the game is foreground and written as `no_modifiers F6`.
3. A hunting key in `[Hunting]`, not `[Key*]`, with chords/negative modifiers and engine actions such as reload/frame analysis.

The settings validator compares all EMMM actions, reserved package bindings F6/F8, and the reload key discovered from `[Hunting]`. Saved user bindings are not silently migrated; default changes only apply to new configurations or after a reset action.

## 7. Minimum verification and regression guide

Use the following matrix when changing the parser, writer, runtime profile, KeyViewer, conflict engine, or switching flow. Implementation status details and outstanding validation work are in [`3dmigoto_gap_status.md`](./3dmigoto_gap_status.md).

- Nested-mod fixtures with UTF-8 BOM, Shift-JIS, mixed CRLF/LF, final/no-final newline, repeated key/back, qualified variables, flow control, and non-ASCII sections.
- Golden round-trip: a no-op save produces identical bytes; a targeted save changes only the requested token.
- Stale-write test: a file changed externally after reading must produce a conflict, not an overwrite.
- Failure injection: temp write, replacement, backup, and rollback failures always leave the original or a recoverable backup.
- GIMI/WWMI/SRMI/ZZMI package matrix: config locations, text namespaces, F6/F8/F10/F12, relative resource paths, and reload grammar.
- Effective-enabled matrix for disabled prefixes on every ancestor and legacy/case variants.
- Matcher fixtures from asset repositories: shared textures, duplicate skin hashes, shader64/resource32, zero sentinel, and ambiguous sentinel.
- In-game smoke test: toggling mods/objects/presets, F10, camera pan, character swap, safe mode, status fallback, and turning the overlay off must not leave stale notifications.

### 7.1 Repository snapshots reviewed

These commits are recorded so claims can be reproduced and updated without relying on a moving “latest” reference:

| Repository                                  | Branch   | Commit snapshot                            |
| ------------------------------------------- | -------- | ------------------------------------------ |
| `bo3b/3Dmigoto`                             | `master` | `4ce5f2f72b2777223d2e809dcdafec514ac98295` |
| `rayshire/3dmigoto`                         | `main`   | `7a1255b45cac2133c4adc4c697cc7241263fa722` |
| `SilentNightSound/GIMI-Package`             | `main`   | `a88633c677766b81290d0de9f91a879f862c2bc0` |
| `SilentNightSound/GI-Model-Importer`        | `main`   | `4232c2679193cad7f15898a20517798560d38153` |
| `SilentNightSound/GI-Model-Importer-Assets` | `main`   | `2039d16d4b64696098ba53cd69888ce967397be9` |
| `SilentNightSound/SR-Model-Importer`        | `main`   | `ecb4a4708bceb5e134bc571a9bbc43fd10c21363` |
| `SilentNightSound/SR-Model-Importer-Assets` | `main`   | `eff6cdf613cb3b07cc241ca3f287abe3b1496b71` |
| `SpectrumQT/WWMI-Package`                   | `main`   | `647462518c1916e04dea1de9048f152326960795` |
| `leotorrez/ZZ-Model-Importer`               | `main`   | `4fb37188b8f1b509e0bfcef877b236a8ade6e4d4` |
| `SpectrumQT/XXMI-Launcher`                  | `main`   | `d56786b8dacb00c35204bff45ff5b8b83bd8962a` |

Ecosystem freshness note: XXMI Launcher currently points to modern SRMI/ZZMI packages that differ from the older standalone repositories in the initial list (`SpectrumQT/SRMI-Package` and `leotorrez/ZZMI-Package`). The older repositories remain useful for guides/tools, but production runtime profiles must follow the package actually installed by the launcher.

## 8. Primary references

- [bo3b/3Dmigoto](https://github.com/bo3b/3Dmigoto) and [Dependencies/d3dx.ini](https://github.com/bo3b/3Dmigoto/blob/master/Dependencies/d3dx.ini)
- [GIMI-Package](https://github.com/SilentNightSound/GIMI-Package), [d3dx.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/d3dx.ini), [API.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/API.ini), [help.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/help.ini), and [KeyBindings.ini](https://raw.githubusercontent.com/SilentNightSound/GIMI-Package/main/GIMI/Core/GIMI/KeyBindings.ini)
- [WWMI-Package](https://github.com/SpectrumQT/WWMI-Package) and [WWMI KeyBindings.ini](https://raw.githubusercontent.com/SpectrumQT/WWMI-Package/main/WWMI/Core/WWMI/KeyBindings.ini)
- [GI-Model-Importer](https://github.com/SilentNightSound/GI-Model-Importer), [GI assets](https://github.com/SilentNightSound/GI-Model-Importer-Assets), [SR importer](https://github.com/SilentNightSound/SR-Model-Importer), and [SR assets](https://github.com/SilentNightSound/SR-Model-Importer-Assets)
- [ZZ-Model-Importer](https://github.com/leotorrez/ZZ-Model-Importer), [XXMI-Launcher](https://github.com/SpectrumQT/XXMI-Launcher), and [rayshire mirror](https://github.com/rayshire/3dmigoto)
- [Leotorrez INI docs](https://leotorrez.github.io/modding/docs/), [Hunting](https://leotorrez.github.io/modding/guides/hunting), [Textures 101](https://leotorrez.github.io/modding/guides/textures-101), and [Shaders 101](https://leotorrez.github.io/modding/guides/shaders-101)
