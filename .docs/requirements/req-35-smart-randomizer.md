# Epic 35: Smart Randomizer & Integrated Game Launcher

## 1. Executive Summary

- **Problem Statement**: (1) Users want variety in their modded gameplay but enabling random mods naïvely creates conflicts (two skins for one character) or breaks visual integrity — a random loadout must respect Object boundaries. (2) Users must manually start the 3DMigoto loader and game separately, often dealing with UAC prompts and timing; a single "Play" button would save friction.
- **Proposed Solution**: Two QoL features: (1) A `suggest_random_mods` backend command that selects one random mod per Object using `rand::seq::SliceRandom`, respects `is_safe` filter, excludes dot-prefix folders, applies via Collections apply machinery with preview → confirm → apply flow; (2) A `launch_game` command that checks for the running loader via `sysinfo`, starts it as Admin if not running, then starts the game EXE with configured `launch_args`, optionally auto-closing EMMM2.
- **Success Criteria**:
  - `suggest_random_mods` returns a random proposal in ≤ 200ms for ≤ 500 Objects; selection algorithm runs in ≤ 10ms even with 10,000 items.
  - Zero mod-per-Object conflicts in any generated loadout — exactly 1 mod per Object with ≥ 1 eligible mod.
  - Safe Mode filter correctly excludes `is_safe = false` mods — 0 NSFW mods in a safe result.
  - Dot-prefix folders (system/fixed mods) are never included in the random pool.
  - `launch_game` issues the process execution command in ≤ 100ms.
  - Apply uses the same atomic Collections machinery — rollback on failure, undo toast on success.
  - Re-rolling generates a different result in ≥ 80% of re-rolls for Objects with ≥ 2 mods.

---

## 2. User Experience & Functionality

### User Stories

#### US-35.1: Integrated Game Launcher (One-Click Play)

As a gamer, I want a single "Play" button that automatically manages the 3DMigoto loader and starts the game, so that the process of starting a modded session is instant and hassle-free.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                                                                                                     |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-35.1.1 | ✅ Positive | Given a configured game with a valid `launcher_path`, when I click "Play", then: (1) `sysinfo` checks if the loader process is already running; (2) if not running, launches it via `RunAs` (Admin) using `powershell start-process -Verb RunAs`; (3) launches the game EXE with any configured `launch_args` (e.g., `-popupwindow`); all within ≤ 100ms of the button click |
| AC-35.1.2 | ✅ Positive | Given the "Auto-Close on Launch" setting is ON, when the game EXE launches successfully, then EMMM2 calls `app.exit(0)` — the mod manager closes automatically                                                                                                                                                                                                               |
| AC-35.1.3 | ❌ Negative | Given the user declines the UAC prompt, then EMMM2 logs `"Launch Cancelled: UAC denied"` and shows a toast "Please allow Admin access when prompted — the loader requires elevated permissions"                                                                                                                                                                              |
| AC-35.1.4 | ❌ Negative | Given the configured `launcher_path` no longer exists on disk, when "Play" is clicked, then a toast shows "Launcher not found — update your game path in Settings" and navigates to the Settings > Games tab                                                                                                                                                                 |
| AC-35.1.5 | ⚠️ Edge     | Given the loader is already running (checked via `sysinfo`), then the loader start step is skipped — only the game EXE is launched                                                                                                                                                                                                                                           |

---

#### US-35.2: Smart Random Loadout Generation

As a user, I want the app to pick one random mod per character with a preview before applying, so that I have full control before any files are touched.

| ID        | Type        | Criteria                                                                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-35.2.1 | ✅ Positive | Given the Randomizer Modal, when I click "Generate New Setup", then `suggest_random_mods` returns exactly 1 mod per Object that has ≥ 1 eligible mod; Objects with 0 eligible mods are excluded from the proposal |
| AC-35.2.2 | ✅ Positive | Given the app is in Safe Mode, then only `is_safe = true` mods are eligible — result contains 0 NSFW mods regardless of pool size                                                                                 |
| AC-35.2.3 | ✅ Positive | Given the generated proposal, then a preview dialog shows "Keqing: Neon Skin, Hu Tao: Casual Outfit…" with thumbnails — clicking "Re-roll" calls `suggest_random_mods` again for a new distinct random selection  |
| AC-35.2.4 | ✅ Positive | Given I click "Apply This Setup", then the Collections apply machinery is used: snapshot → `OperationLock` + `SuppressionGuard` → bulk toggle → undo toast                                                        |
| AC-35.2.5 | ❌ Negative | Given all mods for an Object are `is_safe = false` and Safe Mode is Active, that Object is excluded entirely — no entry in the proposal from that Object                                                          |
| AC-35.2.6 | ❌ Negative | Given another toggle operation is in progress (`OperationLock` held), then clicking "Apply This Setup" is blocked — a toast shows "Cannot apply while another operation is running"                               |
| AC-35.2.7 | ⚠️ Edge     | Given an Object has exactly 1 eligible mod, then that mod is always deterministically selected; no randomness needed                                                                                              |
| AC-35.2.8 | ⚠️ Edge     | Given all mods in the entire library are excluded (Safe Mode ON + all NSFW, or empty library), then the Randomizer shows "No mods available for randomization" empty state — no panic or null proposal            |

---

#### US-35.3: Dot-Prefix Exclusion (System Mods)

As a user, I want system/fixed mods (prefixed with ".") to be immune to the randomizer, so that mandatory mods are never accidentally disabled.

| ID        | Type        | Criteria                                                                                                                                                                                     |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-35.3.1 | ✅ Positive | Given the randomizer pool, folders whose physical name starts with `"."` (e.g., `.EMMM2_System`, `.SharedShaders`) are never included as candidates — they are neither disabled nor replaced |
| AC-35.3.2 | ⚠️ Edge     | Given a mod named `.disabled_skin` (dot + disabled prefix), then it is excluded from the pool both by the dot rule and the is-disabled filter — no double-processing needed                  |

---

### Non-Goals

- No weighted randomness by last-played or usage frequency.
- No "exclude certain characters" option in this epic.
- No persistence of generated proposals — re-opening the modal generates fresh.
- No random selection across multiple active games simultaneously.
- No auto-apply without preview — confirmation step is always required.

---

## 3. Technical Specifications

### Architecture Overview

```
launch_game(game_id) → Result<(), AppError>:
  game = get_game_config(game_id)
  loader_name = Path::new(game.launcher_path).file_name()
  sysinfo::System::new_all() → if !processes_by_name(loader_name).any():
    #[cfg(windows)] powershell start-process {launcher_path} -Verb RunAs
  std::process::Command::new(game.game_exe).args(game.launch_args).spawn()
  if settings.auto_close_on_launch: app.exit(0)

suggest_random_mods(game_id, safe_mode_enabled) → Vec<RandomModProposal>:
  objects = SELECT DISTINCT object_id FROM folders WHERE game_id = ?
  for each object_id:
    candidates = SELECT folder_path, name FROM folders
      WHERE object_id = ? AND game_id = ?
        AND NOT starts_with(folder_name, '.')  // dot-prefix exclusion
        AND (NOT safe_mode_enabled OR is_safe = true)
    if candidates.is_empty(): skip
    winner = candidates.choose(&mut thread_rng())
    proposals.push({ object_id, folder_path, name, thumbnail_path })
  return proposals

Frontend Flow:
  "Play" button → invoke('launch_game', { game_id })
  "Generate New Setup" → invoke('suggest_random_mods', { game_id, safe_mode_enabled })
    → RandomizerModal shows preview (thumbnail + name per Object)
  "Re-roll" → invoke('suggest_random_mods') again
  "Apply This Setup" → invoke('apply_collection_from_paths', { game_id, folder_paths })
    → same machinery as apply_collection (Epic 31): snapshot + bulk toggle + undo toast
```

### Integration Points

| Component         | Detail                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------ |
| sysinfo           | `sysinfo::System::new_all().processes_by_name(loader_name)` — checks if loader is running  |
| Launcher          | `powershell start-process -Verb RunAs` on Windows for elevated launch                      |
| Randomization     | `rand::seq::SliceRandom::choose(&mut thread_rng())`                                        |
| Dot-prefix filter | `!folder_name.starts_with('.')` — applied in Rust query layer, not frontend                |
| OperationLock     | Checked before `apply_collection_from_paths` — returns `AppError::Busy` if held            |
| Apply             | Reuses `apply_collection` machinery (Epic 31) — snapshot + bulk toggle + undo toast        |
| Frontend          | `RandomizerModal.tsx` + `LaunchButton.tsx` — `useSuggestRandomMods`, `useLaunchGame` hooks |

### Security & Privacy

- **`suggest_random_mods` is read-only** — no file mutations; proposals are just `Vec<folder_path>`.
- **Safe Mode filter enforced backend-side** — never trusts frontend-passed `is_safe` values.
- **Launcher uses `std::process::Command`** — no shell=true exec; paths are canonicalized before use.
- **UAC elevation via PowerShell `start-process -Verb RunAs`** — never hardcodes administrator tokens.

---

## 4. Dependencies

- **Blocked by**: Epic 31 (Collections — apply machinery), Epic 30 (Privacy/Safe Mode — `is_safe` flag), Epic 02 (Game Management — `launcher_path`, `game_exe`, `launch_args`).
- **Blocks**: Nothing — leaf feature.
