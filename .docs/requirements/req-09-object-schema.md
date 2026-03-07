# Epic 09: Object Schema & Master Database

## 1. Executive Summary

- **Problem Statement**: Each supported game (Genshin, HSR, ZZZ, WuWa, Endfield) has different mod category taxonomies and official character/weapon names in multiple languages — without a schema-driven system, the UI would hardcode game-specific logic and break for any new game addition.
- **Proposed Solution**: A bundled `schema.json` per game that defines mod categories, stopwords, and aliases — loaded at startup into memory. A complementary Master Database (bundled JSON) maps raw folder name tokens to canonical entity names, used during scan and folder listing to normalize object identities.
- **Success Criteria**:
  - Schema file loads and validates in ≤ 100ms at app boot for any supported game.
  - Object category renders correctly for 100% of schema-defined categories across all 5 games in the test suite.
  - Master DB alias lookup resolves a single unknown folder name in ≤ 5ms using in-memory data structures.
  - App functions with full local-only object names when Master DB JSON is missing — 0 crashes.
  - Schema version mismatch (object category removed in newer version) does not crash — falls back to "Uncategorized".

---

## 2. User Experience & Functionality

### User Stories

#### US-09.1: Game Schema Enforcement

As a user, I want the app to use the correct mod categories for my current game, so that Genshin mods show "Characters/Weapons" and ZZZ mods show their own in-game terminology.

| ID        | Type        | Criteria                                                                                                                                                                                                              |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-09.1.1 | ✅ Positive | Given a specific active game, when the object list renders, then category headers exactly match that game's `schema.json` category definitions — no hardcoded labels                                                  |
| AC-09.1.2 | ✅ Positive | Given a missing or corrupt game-specific schema file, when the app loads, then it falls back to a default schema with generic categories ("Mods") — no blank objectlist                                                  |
| AC-09.1.3 | ❌ Negative | Given a fatally malformed `schema.json` (invalid JSON, missing required fields), when the backend validates it at startup, then the process fails with a clear structured error log — not a silent corrupted state    |
| AC-09.1.4 | ⚠️ Edge     | Given an object whose `category_id` no longer exists in a newer schema version (after an app update), when the objectlist renders, then the object falls into an "Uncategorized" section — no panic, no invisible object |

---

#### US-09.2: Master Database Name Resolution

As a system, I want to resolve raw folder name tokens to canonical official entity names, so that "Raiden" is displayed as "Raiden Shogun" and counts toward the correct object.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                    |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-09.2.1 | ✅ Positive | Given a raw folder name containing a known alias (e.g., "RaidenShogun", "Raiden"), when matched against the Master DB, then it resolves to the canonical name "Raiden Shogun" within ≤ 5ms                                                                  |
| AC-09.2.2 | ✅ Positive | Given a batch of ≥ 100 unknown folder names during scanning, when the matcher tokenizes against the schema stopwords and Master DB whitelist, then ≥ 90% of recognizable names resolve correctly (measured against a 200-entry benchmark dataset)           |
| AC-09.2.3 | ❌ Negative | Given the Master DB JSON file is missing or corrupt, when the scan or folder listing runs, then the app falls back to using raw filesystem folder names as object identities — no crash, no empty object list                                               |
| AC-09.2.4 | ⚠️ Edge     | Given multiple ambiguous aliases mapped to entirely different objects (e.g., "Traveler" could mean Aether or Lumine), when the matcher encounters ambiguity, then it flags the mod for user disambiguation — it does not auto-assign to an arbitrary object |

---

### Non-Goals

- No user-editable schema files via UI in this phase — schemas are bundled assets, updated only via app update (Epic 34).
- No remote schema fetch at runtime; all schemas are embedded in the app bundle.
- No multilingual alias UI display — canonical names are stored and displayed in English.
- No community-uploaded custom schemas in this phase.

---

## 3. Technical Specifications

### Architecture Overview

```
Schema System (Rust)
  ├── schema.json (per game, bundled asset)
  │   ├── categories: [{ id, label, icon, sort_order }]
  │   ├── stopwords: ["mod", "v2", "by", ...]
  │   └── aliases: { "raiden": "Raiden Shogun", "hu_tao": "Hu Tao" }
  ├── services/schema/loader.rs
  │   └── load_schema(game_type) → GameSchema (cached in Arc<RwLock<HashMap<GameType, GameSchema>>>)
  └── services/schema/matcher.rs
      └── resolve_name(raw_tokens) → Option<CanonicalName>

Master DB (Rust)
  └── master_db.json (bundled, per game)
      └── entity_list: [{ id, canonical_name, aliases: [...], category_id }]
      → loaded into HashMap<String, EntityRecord> at startup

Frontend
  └── useGameSchema() → invoke('get_schema', { gameType }) → GameSchema JSON
      └── Driven by React Query, rehydrated on game switch
```

### Integration Points

| Component     | Detail                                                                                        |
| ------------- | --------------------------------------------------------------------------------------------- |
| Schema Load   | `tauri::include_str!("assets/schemas/{game}.json")` — embedded at compile time                |
| Schema Cache  | `Arc<RwLock<HashMap<GameType, GameSchema>>>` — loaded once on bootstrap, read-only after      |
| Master DB     | `HashMap<String (alias_key), EntityRecord>` — built at startup from bundled JSON              |
| Scan Engine   | Passes raw folder name tokens through `matcher.rs` during Epic 25 scan (used in Epic 26)      |
| Frontend Hook | `useGameSchema(gameType)` → React Query → `invoke('get_schema')` — invalidated on game switch |

### Security & Privacy

- **Schema files are bundled read-only assets** — no schema is loaded from user-writable file system paths; no path injection risk.
- **Alias lookup uses a read-only `HashMap`** — no user input mutates the Master DB in memory; only the scan process queries it.
- **Ambiguous alias disambiguation** presents a list of resolution options to the user — no file is auto-moved to an incorrect object folder based on a heuristic guess.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — schema loaded at startup), Epic 02 (Game Management — active `GameType` determines which schema to load).
- **Blocks**: Epic 06/07 (ObjectList — category rendering), Epic 25 (Scan Engine — stopword tokenization), Epic 26 (Deep Matcher — alias resolution).
