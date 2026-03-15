# Epic 30: Privacy & Safe Mode

## 1. Executive Summary

- **Problem Statement**: Users manage mods with varying content sensitivity. Opening the app on stream or in public risks displaying NSFW thumbnails and names. A fast, trustworthy privacy layer is required to prevent accidental exposure.
- **Proposed Solution**: A global Safe Mode toggle (shield icon) backed by `safe_mode` in the backend `AppSettings` (SQLite). Features include: visual masking (blur + obfuscated names) for out-of-corridor mods, auto-classification based on folder name keywords, strict Dual Guard isolation (frontend masking + backend exclusion from queries/counts), and an Argon2-hashed PIN gate.
- **Success Criteria**:
  - [x] Safe Mode toggle applies visual masking and obfuscated names instantly.
  - [x] Auto-classification accurately tags new mods containing restricted keywords during scan and writes to `info.json`.
  - [x] PIN verification performed backend-side using `argon2` crate with constant-time comparison.
  - [x] Brute force limited to 5 failed attempts before a 60s memory-backed lockout.
  - [x] App ALWAYS launches in SFW mode (Safe Mode: Enabled) regardless of previous session state.
  - [x] Dual Guard guarantees 0 leakage — NSFW mods are disabled and hidden from counts in SFW mode.

---

## 2. User Experience & Functionality

### User Stories

#### US-30.1: Toggle Global Safe Mode (Dual Guard)

As a user, I want a quick global toggle to hide sensitive mods, so that I can safely open the app in public without risk of exposure.

| ID        | Type        | Criteria                                                                                                                                                                                                                               |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.1.1 | ✅ Positive | Given the Safe Mode shield icon in the top bar, when clicked (without a PIN set), then Safe Mode becomes Active; `safeMode` state in Zustand updates.                                                                                  |
| AC-30.1.2 | ✅ Positive | Given Safe Mode is Active, then any folder with `is_safe: false` in FolderGrid has its thumbnail replaced with a blurred placeholder and its name masked to "[Hidden Mod]" — applied via CSS `filter: blur(12px)`.                     |
| AC-30.1.3 | ✅ Positive | Given Safe Mode is Active, then backend queries for objectlist counts automatically filter by the active corridor (`COALESCE(is_safe, 1) = ?`), preventing inference of NSFW mods from numerical mismatches.                           |
| AC-30.1.4 | ❌ Negative | Given Safe Mode is Active and a PIN is set, when the shield is clicked to disable, then `PinEntryModal` opens — Safe Mode does NOT disable until a correct PIN is entered.                                                              |
| AC-30.1.5 | ✅ Positive | Given the app is launched, then Safe Mode is ALWAYS enabled by default (`safe_mode.enabled = true` in `ConfigService::load_from_db`) — preventing accidental exposure of NSFW content on startup.                                      |

---

#### US-30.2: Privacy Tagging & Auto-Classification

As a user, I want the system to automatically tag new mods based on keywords, so that I don't need to manually verify every imported mod.

| ID        | Type        | Criteria                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.2.1 | ✅ Positive | Given a folder is scanned, if its `display_name` contains keywords from `safe_mode_keywords`, then the system automatically tags it as `is_safe = false` and writes this to its disk `info.json`. |
| AC-30.2.2 | ✅ Positive | Given the context menu, when I manually toggle "Mark as NSFW", then `UPDATE mods SET is_safe = false` executes and the value is written to the physical folder's `info.json`.                   |
| AC-30.2.3 | ⚠️ Edge     | Given I mark a mod as safe while Safe Mode is Active, then it remains hidden from disk (disabled) until corridor switch restores it.                                                            |

---

#### US-30.3: Safe Mode Lock (PIN Security)

As a user, I want to lock the Safe Mode toggle with a PIN, so that others cannot easily bypass the privacy filter.

| ID        | Type        | Criteria                                                                                                                                                                                  |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.3.1 | ✅ Positive | Given the Settings, when I enter a 6-digit PIN and click Set, it is hashed with Argon2 and stored in the backend configuration (`app_settings` table).                                    |
| AC-30.3.2 | ✅ Positive | Given a PIN is set and Safe Mode is Active, when I click to disable, the `PinEntryModal` opens. On correct PIN, `verify_pin()` returns `true` and Safe Mode is disabled.                  |
| AC-30.3.3 | ❌ Negative | Given I enter an incorrect PIN, then a visual shake animation plays and an error message shows attempts remaining.                                                                        |
| AC-30.3.4 | ❌ Negative | Given 5 consecutive incorrect PIN attempts, then the PIN entry locks for 60 seconds. Lockout state is managed in backend `PinGuardState` (memory) and persisted to `app_settings` via DB. |
| AC-30.3.5 | ⚠️ Edge     | Given the user forgets their PIN, they must manually delete the `safe_mode` JSON blob from the `app_settings` SQLite table to reset it.                                                   |

---

#### US-30.4: Atomic Switch Corridor (Corridor Handoff)

As a user, I want the system to automatically disable opposing-mode mods and restore my previously active mods when I switch modes, so that I don't need to manually toggle mods each time.

| ID        | Type        | Criteria                                                                                                                                                                                                                         |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.4.1 | ✅ Positive | Given I switch modes, the system: (1) Disables enabled sub-mods (excluding depth-1 folders) in the leaving corridor with `disabled_reason = 'SYSTEM'`, (2) Restores the target corridor by querying mods where `disabled_reason = 'SYSTEM'` matching target context. |
| AC-30.4.2 | ✅ Positive | Given a switch is triggered, a `ModeSwitchConfirmModal` shows a preview of the "Leaving State" and "Destination State" (Retrieved via `get_system_disabled_preview_mods`).                                                            |
| AC-30.4.3 | ✅ Positive | Given the switch completes, then ObjectList and FolderGrid re-fetch via TanStack Query invalidation.                                                                                                                             |
| AC-30.4.5 | ⚠️ Edge     | Given `PrivacyManager` renames mod folders during switch, it also updates `folder_path` in `mods` table and `objects.folder_path` for top-level directories to maintain sync.                                                  |

---

#### US-30.5: Mutually Exclusive Corridor Enforcement

As a user, I want opposite-mode mods to be impossible to enable while the wrong mode is active, so that I can't accidentally leak content.

| ID        | Type        | Criteria                                                                                                                                                                                           |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.5.1 | ✅ Positive | Given Safe Mode is active, the ObjectList shows ALL objects but counts reflect ONLY mods in the Safe Corridor.                                                                                     |
| AC-30.5.2 | ✅ Positive | Given Safe Mode is active, attempting to manually enable an NSFW mod from FolderGrid (or vice-versa) is prevented by Dual Guard logic — opposite corridor mods are physically disabled (prefixed). |
| AC-30.5.3 | ✅ Positive | Given a collection with `is_safe_context` opposite to current mode, it cannot be applied until a corridor switch occurs.                                                                           |

---

### Non-Goals

- Safe Mode focuses on UI visibility (masking) and disk-level corridor separation (disabling via prefix). It does not encrypt files.
- No auto-lock timer based on idle activity.
- No remote PIN sync.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: ConfigService (pin_guard.rs)
// Hashing PINs via argon2 crate; lockouts stored in memory + DB
fn verify_hash(hash: &str, pin: &str) -> bool {
    Argon2::default().verify_password(pin.as_bytes(), &parsed_hash).is_ok()
}

// Corridor Switch: PrivacyManager (mod.rs)
// 1. Disable sub-mods (Exclude depth-1 Object containers; batch fs::rename + path updates; sets `disabled_reason = 'SYSTEM'`)
// 2. Restore Target (Queries mods where `disabled_reason = 'SYSTEM'` for target context, renames and clears reason)
pub async fn switch_mode(target_mode: Mode, ...) -> Result<CorridorSwitchResult, String>;
```

### Integration Points

| Component        | Detail                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------- |
| Safe Mode State  | `useAppStore().safeMode` synchronized with backend `get_settings`.                                         |
| Masking UI       | `FolderCard.tsx` uses `filter: blur(12px)` and masked text for `is_safe: false` mods.                      |
| Auto-Tagging     | `commit_scan_results` checks `safe_mode_keywords` and updates mod `info.json` upon import.                 |
| Path Maintenance | `PrivacyManager` updates `objects.folder_path` and `mods.folder_path` (including nested children) on rename. |
| Corridor Handoff | `set_safe_mode_enabled` command triggers `PrivacyManager` atomic swap setting/clearing `disabled_reason = 'SYSTEM'` + Query Cache invalidation.           |
| Nested Mods      | Target queries handle bringing back mod states inside ContainerFolders precisely based on `disabled_reason`.   |

### Security & Privacy

- **Argon2 / Constant-Time**: Prevents brute-forcing and timing attacks.
- **SFW-on-Launch**: Guaranteed in `ConfigService::load_from_db`.
- **Dual Guard**: Combines frontend visual scrubbing with backend filesystem disabling.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (Store setup), Epic 25 (Scan Engine).
- **Blocks**: Epic 31 (Collections).
