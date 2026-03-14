# Epic 30: Privacy & Safe Mode

## 1. Executive Summary

- **Problem Statement**: Users manage mods with varying content sensitivity. Opening the app on stream or in public risks displaying NSFW thumbnails and names. A fast, trustworthy privacy layer is required to prevent accidental exposure.
- **Proposed Solution**: A global Safe Mode toggle (shield icon) backed by `safeMode: bool` in Zustand store + `store.json`. Features include: visual masking (blur + obfuscated names) for `is_safe = false` mods, auto-classification based on folder name keywords, strict Dual Guard isolation (frontend masking + backend exclusion from queries/counts), and an Argon2-hashed PIN gate.
- **Success Criteria**:
  - Safe Mode toggle applies visual masking to all `is_safe = false` mods in ≤ 100ms.
  - Auto-classification accurately tags new mods containing restricted keywords during scan in ≤ 50ms per folder.
  - PIN verification is performed backend-side (never exposed to frontend) — brute force is limited to 5 failed attempts before a 60s lock.
  - Dual Guard guarantees 0 leakage — NSFW mods are invisible in SFW mode exports, and their existence is hidden from objectlist `enabled_count` metrics.
  - Toggle states and manually set `is_safe` flags properly sync to SQLite and the mod's portable `info.json` within ≤ 200ms.

---

## 2. User Experience & Functionality

### User Stories

#### US-30.1: Toggle Global Safe Mode (Dual Guard)

As a user, I want a quick global toggle to hide sensitive mods, so that I can safely open the app in public without risk of exposure.

| ID        | Type        | Criteria                                                                                                                                                                                                |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.1.1 | ✅ Positive | Given the Safe Mode shield icon in the top bar, when clicked (without a PIN set), then Safe Mode becomes Active; `useAppStore().safeMode = true` is set instantly; persisted to `store.json`            |
| AC-30.1.2 | ✅ Positive | Given Safe Mode is Active, then any mod with `is_safe = false` has its thumbnail replaced with a blurred placeholder and its name masked to "[Hidden Mod]" — applied via CSS `filter: blur(12px)`       |
| AC-30.1.3 | ✅ Positive | Given Safe Mode is Active (Dual Guard Backend), then backend queries for objectlist `enabled_count` automatically inject `AND is_safe = true`, preventing inference of NSFW mods from numerical mismatches |
| AC-30.1.4 | ❌ Negative | Given Safe Mode is Active and a PIN is set, when the shield is clicked to disable, then `PinEntryModal` opens — Safe Mode does NOT disable until a correct PIN is entered                               |
| AC-30.1.5 | ⚠️ Edge     | Given the app is closed while Safe Mode is Active, then on next launch Safe Mode is restored to Active strictly before the UI renders — no transient pop-in where NSFW content flashes on screen        |

---

#### US-30.2: Privacy Tagging & Auto-Classification

As a user, I want the system to automatically tag new mods based on keywords, so that I don't need to manually verify every imported mod.

| ID        | Type        | Criteria                                                                                                                                                                                            |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.2.1 | ✅ Positive | Given a folder is imported, if its physical `folder_name` contains keywords from `settings.safe_mode_keywords` (e.g., "Nude", "NSFW"), then the system automatically tags it as `is_safe = false`   |
| AC-30.2.2 | ✅ Positive | Given the context menu or Metadata Section, when I manually toggle "Mark as NSFW", then `UPDATE folders SET is_safe = false` executes AND the value is written to the physical folder's `info.json` |
| AC-30.2.3 | ⚠️ Edge     | Given I mark a mod as safe while Safe Mode is Active, then it remains hidden UI-side until Safe Mode is disabled (the global `safeMode` flag prevails to prevent accidental unmasking)              |

---

#### US-30.3: Safe Mode Lock (PIN Security)

As a user, I want to lock the Safe Mode toggle with a PIN, so that others cannot easily bypass the privacy filter.

| ID        | Type        | Criteria                                                                                                                                                                                                     |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-30.3.1 | ✅ Positive | Given the Privacy Settings tab, when I enter a PIN and click Set, it is hashed with Argon2 and stored in the backend configuration — the raw PIN is never stored or logged                                   |
| AC-30.3.2 | ✅ Positive | Given a PIN is set and Safe Mode is Active, when I click to disable, the `PinEntryModal` opens. On correct PIN, `verify_pin()` returns `true` and Safe Mode is disabled                                      |
| AC-30.3.3 | ❌ Negative | Given I enter an incorrect PIN, then a visual "shake" animation plays — no hint or length indicator is provided                                                                                              |
| AC-30.3.4 | ❌ Negative | Given 5 consecutive incorrect PIN attempts, then the PIN entry locks for 60 seconds — a countdown "Try again in Xs" is shown, managed by a backend state timer to prevent client-side bypass                 |
| AC-30.3.5 | ⚠️ Edge     | Given the user forgets their PIN, they follow the documented manual recovery (deleting the `pin_hash` from the physical JSON config), which requires OS file-level access, verifying local machine ownership |

---

#### US-30.4: Atomic Switch Corridor (PrivacyManager)

As a user, I want the system to automatically disable opposing-mode mods and restore my previously active mods when I switch modes, so that I don't need to manually toggle mods each time.

| ID        | Type        | Criteria                                                                                                                                                                                                   |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.4.1 | ✅ Positive | Given I switch Safe→Unsafe, the system: (a) snapshots SFW mods' enabled status, (b) disables ALL SFW mods, (c) restores previously-snapshotted NSFW mods, (d) batch renames folders on disk               |
| AC-30.4.2 | ✅ Positive | Given I switch Unsafe→Safe, the system: (a) snapshots NSFW mods' enabled status, (b) disables ALL NSFW mods, (c) restores previously-snapshotted SFW mods, (d) batch renames folders on disk              |
| AC-30.4.3 | ✅ Positive | Given the switch completes, then ObjectList re-fetches (opposite-mode objects show zeroed counts) and FolderGrid re-fetches (backend filter applies)                                                       |
| AC-30.4.4 | ⚠️ Edge     | Given a mod's physical folder is missing during batch rename, the system logs a warning and continues (soft fail) — one broken mod does not abort the entire switch                                         |
| AC-30.4.5 | ⚠️ Edge     | Given PrivacyManager renames a top-level folder (Flat Mod), it also updates both the `objects.folder_path` and any child `mods.folder_path` records in the database to maintain DB↔FS sync                 |
| AC-30.4.6 | ⚠️ Edge     | Given a mod is deeply nested inside an object folder, its `is_safe` status dynamically inherits its parent's privacy flag (`COALESCE(o.is_safe, m.is_safe, 1)`), guaranteeing 0 cross-corridor leakage      |

---

#### US-30.5: Mutually Exclusive Corridor Enforcement

As a user, I want opposite-mode mods to be impossible to enable while the wrong mode is active, so that I can't accidentally leak content.

| ID        | Type        | Criteria                                                                                                                                                                                                   |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-30.5.1 | ✅ Positive | Given Safe Mode is active, the ObjectList shows ALL objects but zeroes `mod_count` and `enabled_count` for unsafe objects (is_safe=0). Unsafe objects appear "empty" in the navigation                      |
| AC-30.5.2 | ✅ Positive | Given a user right-clicks an **enabled** mod and selects "Toggle Safe", a warning toast is shown: "Disable this mod before changing its privacy status"                                                    |
| AC-30.5.3 | ✅ Positive | Given a collection with `is_safe_context` opposite to the current mode, the Apply button is disabled and a warning is shown                                                                                |
| AC-30.5.4 | ✅ Positive | Given queries for Safe Mode stats or enabled mods, `WHERE COALESCE(o.is_safe, m.is_safe, 1)` strictly governs visibility to maintain mutually exclusive isolated corridors                               |

---

### Non-Goals

- Safe Mode focuses on UI visibility (Dual Guard masking). It does not encrypt, physically move, or forcefully disable `is_safe = false` mods in the game directory unless managed via Collections.
- No auto-lock timer based on idle activity.
- No remote PIN sync or cloud backup recovery.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend State
struct PrivacyManager {
    pin_hash: Option<String>, // Argon2 hash
    keywords: Vec<String>,
}

// Auto-Classification Logic (During Scan)
fn evaluate_safety(folder_name: &str, keywords: &[String]) -> bool {
    let normalized = folder_name.to_lowercase();
    !keywords.iter().any(|k| normalized.contains(&k.to_lowercase()))
}

// PIN Verification Flow
fn verify_pin(input: &str) -> Result<bool, AppError> {
    // 1. Check rate limit map (fail if locked)
    // 2. Hash input via Argon2 and constant-time compare against stored hash
    // 3. Increment fail counter if mismatched; lock for 60s if fails >= 5
    // 4. Return bool
}
```

### Integration Points

| Component       | Detail                                                                                        |
| --------------- | --------------------------------------------------------------------------------------------- |
| Safe Mode State | `useAppStore().safeMode` (Zustand) + `store.json` (Tauri plugin-store).                       |
| Masking CSS     | Applied conditionally in `FolderCard` using `filter: blur(12px)`.                             |
| Auto-Tagging    | Integrated into Epic 25 (Scan Engine) and Epic 26 (Deep Matcher) import pipelines.            |
| Database Sync   | Changing `is_safe` updates SQLite (`folders` table) and writes to local `info.json` portable. |
| Hashing         | PIN hashed using Rust's `argon2` crate; frontend never holds the hash.                        |

### Security & Privacy

- **Argon2 Hashing**: Resists GPU brute-forcing.
- **Constant-Time Comparison**: Mitigates timing attacks during PIN verification.
- **Backend Rate Limit**: 60s lockout enforced server-side via memory state.
- **Dual Guard Exclusion**: Prevents data leakage via UI metrics (e.g., hidden files counting toward total enabled stats).

---

## 4. Dependencies

- **Blocked by**: Epic 01 (Store setup), Epic 04 (Settings - Privacy Manager UI hook), Epic 25 (Scan Engine).
- **Blocks**: Epic 31 (Collections - filters context based on Safe Mode state).
