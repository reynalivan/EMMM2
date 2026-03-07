# Epic 26: Deep Matcher & Auto-Categorization

## 1. Executive Summary

- **Problem Statement**: Raw scan results contain folder names like `[V1.2]_Cool_Hu_Tao_Mod_by_Author` — without an intelligent categorization pipeline, every mod must be manually assigned an Object, impractical for libraries of hundreds of mods. Naïve fuzzy matching produces too many false positives.
- **Proposed Solution**: A staged matching engine with two modes (`Quick` and `FullScoring`), five signal layers (Hash, Alias, Deep Content, Token, Direct Name), evidence-gated acceptance, negative evidence penalties, a GameBanana enrichment reranker, an optional LLM fallback, and a manual user review interface before DB commit. Designed for zero false positives over accuracy.
- **Success Criteria**:
  - Quick pipeline auto-assigns the correct Object for ≥ 70% of a 200-mod benchmark dataset without AI.
  - Full pipeline (with AI reranker) achieves ≥ 85% correct auto-assignment on the same 200-mod benchmark.
  - `NeedsReview` mods always bubble up in the review table — 0 low-confidence mods silently committed.
  - Quick pipeline processes one mod in ≤ 50ms (in-memory index lookups + bounded INI reads).
  - `ScanReviewModal` loads with all proposed mappings in ≤ 500ms for 500-mod scan results.

---

## 2. User Experience & Functionality

### User Stories

#### US-26.1: Heuristic Matching (Staged Pipeline)

As a system, I want to run a staged pipeline of mechanical matching strategies, so that most mods are categorized correctly without an AI call.

| ID        | Type        | Criteria                                                                                                                                                                                           |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-26.1.1 | ✅ Positive | Given a folder named exactly after a known entity alias (e.g., `Albedo`), then the Alias Strict stage (L0) matches it with score ≥ 12, `Confidence::High`, status `AutoMatched`                    |
| AC-26.1.2 | ✅ Positive | Given a noisy folder `[V1.2] Cool_Hu_Tao_Mod by Author`, when stopword-filtered, the token `hutao` matches the golden corpus; the Token Overlap (L2) stage assigns score ≥ threshold + margin      |
| AC-26.1.3 | ✅ Positive | Given a folder with discoverable `.ini` files, when hashes in those files map to a unique entry in `hash_index`, then Hash stage (L1) returns `AutoMatched` with `Confidence::High`                |
| AC-26.1.4 | ❌ Negative | Given a folder name that matches only via `DirectNameSupport` (L1 name token hit), then the status is NEVER `AutoMatched` — L1 cannot be the sole reason; it only contributes score                |
| AC-26.1.5 | ❌ Negative | Given a folder with no scoreable signals (all stages below threshold), then the result is `NoMatch` — the folder is placed in Uncategorized, never silently auto-committed                         |
| AC-26.1.6 | ⚠️ Edge     | Given two candidates both have Primary Evidence and score difference < 1.0, then the result is forced to `NeedsReview` even if margin acceptance would otherwise pass — ultra-close tie prevention |
| AC-26.1.7 | ✅ Positive | Given the matched object is `Character`, then the post-match Skin/Variant Resolver checks `folder_tokens` against `db.official_skins`; if a trigger keyword matches, `detected_skin` is assigned   |

---

#### US-26.2: Deep Content Scan (Primary Fallback When Hashes Missing)

As a system, I want to extract tokens from subfolder names, file stems, and INI content when hashes are absent, so that mods without hash data can still be matched.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                             |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-26.2.1 | ✅ Positive | Given a mod folder with no `.ini` files or unrecognized hashes, when Deep Content Scan runs (L3), then subfolder names and file stems are tokenized, normalized, and matched against entity tokens                                                                   |
| AC-26.2.2 | ✅ Positive | Given `.ini` files present, then section headers `[TextureOverrideDilucVB]` are PascalCase-split and stripped of common prefixes (`TextureOverride`, `ShaderOverride`, `Resource`, `CommandList`) — residual `diluc`, `vb` tokens contribute to `ini_section_tokens` |
| AC-26.2.3 | ✅ Positive | Given INI key-value lines, then key names (LHS) and RHS values that look like file paths (containing `.dds`, `.png`, `.buf`, `.ini`) are tokenized — path stems and segments contribute to `ini_content_tokens`                                                      |
| AC-26.2.4 | ❌ Negative | Given purely numeric tokens, stopwords (`mod`, `skin`, `texture`, `override`, `resource`, `commandlist`), or keys in the `ini_key_blacklist` (`run`, `handling`, `drawindexed`, `vb`, `ib`, `ps`, `vs`), then those tokens are skipped                               |
| AC-26.2.5 | ⚠️ Edge     | Given a malformed or binary `.ini` file, then UTF-8 decoding is attempted; if it fails, lossy decoding is used; the scanner never panics — it moves on to the next file                                                                                              |

---

#### US-26.3: Evidence Gate (Zero False Positives)

As a system, I want to block `AutoMatched` status unless the best candidate has at least one Primary Evidence signal, so that high-score but evidence-free matches are still flagged for review.

| ID        | Type        | Criteria                                                                                                                                                                                                                                           |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-26.3.1 | ✅ Positive | Given score + margin acceptance passes, AND the best candidate has `HashOverlap.overlap ≥ 1`, then `AutoMatched` is returned                                                                                                                       |
| AC-26.3.2 | ✅ Positive | Given Primary Evidence comes from Deep structure (`deep_ratio ≥ 0.12` OR `deep_hits ≥ 2`), then `AutoMatched` is permitted                                                                                                                         |
| AC-26.3.3 | ✅ Positive | Given Primary Evidence from INI (`ini_section_hits + ini_content_hits ≥ 1`), then `AutoMatched` is permitted                                                                                                                                       |
| AC-26.3.4 | ❌ Negative | Given only `DirectNameSupport` reasons (L1 name/tag token hit) contribute to primary evidence — this is explicitly NOT sufficient; `DirectNameSupport` never satisfies the Evidence Gate                                                           |
| AC-26.3.5 | ⚠️ Edge     | Given a mod pack folder with ≥ 2 candidates each having Primary Evidence and both scoring above `review_min_score`, then `NeedsReview` is forced regardless of margin — prevents a pack folder from being auto-assigned to the wrong single entity |

---

#### US-26.4: Negative Evidence (Conflict Penalty)

As a system, I want to penalize candidates whose token set doesn't explain strong tokens in the observed signal, so that cross-contaminated mod packs don't auto-match wrongly.

| ID        | Type        | Criteria                                                                                                                                                                                                                           |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-26.4.1 | ✅ Positive | Given a folder signal contains a rare strong token (`df <= 3`) that belongs to a different entity (not candidate A), then candidate A's score decreases by `2.5 * foreign_strong_hits` (FullScoring) — capped at 10 points penalty |
| AC-26.4.2 | ✅ Positive | Given two candidates both have Primary Evidence and `best_score - second_score < margin`, then the status is forced to `NeedsReview` even if acceptance criteria otherwise pass                                                    |
| AC-26.4.3 | ⚠️ Edge     | Given `foreign_strong_hits = 0`, the penalty is zero — no undue reduction for mods with clean signals                                                                                                                              |

---

#### US-26.5: GameBanana Enrichment (Mechanical Rerank)

As a system, I want to fetch official mod metadata from the GameBanana API when a GB URL is detected in mod signals, so that mods with obscure folder names can still match correctly.

| ID        | Type        | Criteria                                                                                                                                                                                         |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-26.5.1 | ✅ Positive | Given a GB URL detected in `FolderSignals` (from `info.json` or INI comment), then `api.gamebanana.com/Core/Item/Data` is queried with 3s timeout; official `mod_name` and category are returned |
| AC-26.5.2 | ✅ Positive | Given the API's `mod_name` exactly matches an entity name or alias in the Master DB, then a `+25.0` point bonus is awarded to that candidate with reason `GameBananaEnrichment`                  |
| AC-26.5.3 | ✅ Positive | Given GB API reports category "Skins" but candidate `object_type` is "Weapon", a `-15.0` penalty is applied — category mismatch guard                                                            |
| AC-26.5.4 | ❌ Negative | Given the GameBanana API is unreachable (timeout > 3s or network error), then the rerank stage is skipped; pipeline falls back to mechanical-only result — no hang, no crash                     |

---

#### US-26.6: AI Reranker Fallback (Optional, Default OFF)

As a system, I want to use an external LLM to break ties after all mechanical stages, so that `NeedsReview` volume is minimized for ambiguous mods.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                      |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-26.6.1 | ✅ Positive | Given `opts.ai_enabled = true` AND status is `NeedsReview` after all L3 stages, then a structured JSON prompt is sent to the configured `AiProvider` containing: folder tokens, deep tokens sample, hashes, section tokens, and top-k candidates with reasons |
| AC-26.6.2 | ✅ Positive | Given the LLM responds with `{ "entry_id": score_0_1 }` and `ai_best ≥ 0.7` AND `(ai_best - ai_second) ≥ 0.15`, then the top candidate is upgraded to `AutoMatched`                                                                                           |
| AC-26.6.3 | ❌ Negative | Given `opts.ai_enabled = false` (default), then the AI stage is never invoked — no API call, no delay                                                                                                                                                         |
| AC-26.6.4 | ❌ Negative | Given the LLM returns invalid JSON or a retriable error, then the candidate retains its mechanical `NeedsReview` status — the AI failure is logged at `warn` level and does not block the rest of the batch                                                   |
| AC-26.6.5 | ⚠️ Edge     | Given the same signals hash + DB version were previously AI-evaluated, then the cached result is used — no redundant API call per batch session                                                                                                               |

---

#### US-26.7: Manual Review Interface

As a user, I want to see all proposed auto-categorizations and correct any mistakes before they are committed, so that my library is never corrupted by wrong guesses.

| ID        | Type        | Criteria                                                                                                                                                                                                          |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-26.7.1 | ✅ Positive | Given the scan pipeline finishes, then `ScanReviewModal` opens with a table sorted by confidence ascending (lowest confidence first): `{folder_name, proposed_object, confidence, status, reason_summary}`        |
| AC-26.7.2 | ✅ Positive | Given a `NeedsReview` row, when I click the "Target Object" dropdown, then I can select any Object from the active game's schema — my selection overrides the proposed mapping                                    |
| AC-26.7.3 | ✅ Positive | Given I close the modal without committing, then no DB changes are made — all `ScanResult` data is discarded from memory cleanly                                                                                  |
| AC-26.7.4 | ⚠️ Edge     | Given a `NoMatch` row, I can still manually assign it to any Object (drag from Uncategorized) — it is never silently hidden                                                                                       |
| AC-26.7.5 | ✅ Positive | Given the ScanReviewModal, the user can perform Bulk Actions (Toggle Disable, Delete) via checkboxes, Inline Edit the source folder name, and Open Folder location directly from the table row                    |
| AC-26.7.6 | ✅ Positive | Given the ScanReviewModal, folder names are displayed virtually with a `DISABLED ` prefix (assumed safe default for new mods) simulating normalization, but the physical filesystem rename only occurs on Confirm |

---

### Non-Goals

- No AI model run locally — only external API calls (OpenAI / Anthropic-compatible providers).
- No background or continuous categorization — Deep Matcher runs only after a manual full scan.
- No auto-commit — every scan goes through `ScanReviewModal` before writing to DB.
- No hash equivalence resolution between different 3DMigoto games (hashes are game-scoped).

---

## 3. Technical Specifications

### Architecture Overview

```
MatchMode: Quick | FullScoring
MatchStatus: AutoMatched | NeedsReview | NoMatch
Confidence: High | Medium | Low

MatchResult {
  status: MatchStatus,
  best: Option<Candidate>,
  candidates_topk: Vec<Candidate>,   // top_k=5, sorted score desc → name asc
  evidence: Evidence {
    matched_hashes: Vec<String>,      // max 50, sorted
    matched_tokens: Vec<String>,      // max 50, sorted
    matched_sections: Vec<String>,    // max 50, sorted
    scanned_ini_files: usize,
    scanned_name_items: usize,
  },
  summary: String,  // "Matched by Hash + Alias" or "Ambiguous: Keqing vs Raiden"
}

Candidate {
  entry_id: String,
  name: String, object_type: String,
  score: f32,          // clamped [0.0, 100.0]
  confidence: Confidence,
  reasons: Vec<Reason>, // max 12 per candidate, deterministic order
}

Reason (enum):
  HashOverlap { overlap: u32, unique_overlap: u32 }
  AliasStrict { alias: String }
  DirectNameSupport { token: String }    // supporting only, NOT primary evidence
  TokenOverlap { ratio: f32 }
  DeepNameToken { token: String }
  IniSectionToken { token: String }
  IniContentToken { token: String }
  NegativeEvidence { foreign_strong_hits: u32 }
  AiRerank { ai_score: f32 }
  MechanicalRerank { reason: String }
  GameBananaEnrichment
```

---

### QUICK Mode Pipeline (fast, per-mod < 50ms)

**Budgets**: INI scan — 2 files max, root only, 256KB/file · Deep tokens — depth=1, ≤ 150 name items

| Stage | Name          | Signal                                                                  | Accept Threshold | Margin  |
| ----- | ------------- | ----------------------------------------------------------------------- | ---------------- | ------- |
| Q1    | Hash Sniff    | `extract_hashes_from_ini_text` from root INI → `hash_index` lookup      | `T=10.0`         | `M=6.0` |
| Q2    | Alias Strict  | Folder tokens only; all alias tokens must be in tokenset; `+12` per hit | `T=12.0`         | `M=6.0` |
| Q3    | Deep L3-lite  | Subfolder names + file stems (depth=1) + root INI section/key tokens    | `T=14.0`         | `M=4.0` |
| Q4    | Token Overlap | Folder tokens vs entry tokens; simple ratio `+12 * ratio`               | `T=12.0`         | `M=4.0` |
| Q5    | Direct Name   | Name/tag token in folder tokens; `+4` name, `+2` tag; **no accept**     | —                | —       |

**End decision**: if accepted → `AutoMatched` · else if `score ≥ 10.0` → `NeedsReview` · else `NoMatch`

---

### FULL SCORING Pipeline (accurate, per-mod ≤ 200ms)

**Budgets**: INI scan — 10 files max, depth=3, 1MB total · Deep tokens — depth=3, ≤ 500 name items

| Stage | Name                 | Signal                                                                                       | Accept Threshold | Margin  |
| ----- | -------------------- | -------------------------------------------------------------------------------------------- | ---------------- | ------- |
| F1    | Hash Scan            | All INI files in budget → hash_index                                                         | `T=10.0`         | `M=4.0` |
| F2    | Alias Strict (early) | Folder tokens only                                                                           | `T=12.0`         | `M=4.0` |
| F3    | Deep Content Scan    | Subfolder + file stems (depth=3) + INI section + LHS key + RHS path tokens                   | `T=16.0`         | `M=3.0` |
| F3A   | Alias Re-check       | Re-apply alias after deep/INI tokens collected                                               | `T=12.0`         | `M=4.0` |
| F4    | Weighted Token       | IDF-lite: `w(t)=ln((N+1)/(df(t)+1))+1`; `ratio = sum(w ∩)/max(sum(w folder),ε)`; `+12*ratio` | `T=14.0`         | `M=3.0` |
| F5    | Direct Name          | Name `+2`, tag `+1`; caps 6/4; **no accept**                                                 | —                | —       |
| F6    | Negative Evidence    | `score -= 2.5 * foreign_strong_hits` (cap 10)                                                | —                | —       |
| F7    | GameBanana Rerank    | `+25` exact name, `-15` category mismatch, `+6` description overlap                          | —                | —       |
| F8    | AI Rerank            | if `ai_enabled` AND `NeedsReview`; accept if `ai_best ≥ 0.7 AND gap ≥ 0.15`                  | —                | —       |
| F9    | Root Folder Rescue   | Last-resort: use normalized root folder name if still `NoMatch`                              | —                | —       |

**End decision**: if accepted → `AutoMatched` · else if `score ≥ 12.0` → `NeedsReview` · else `NoMatch`

---

### Scoring Rules & Evidence Gate

```
# Hash scoring (quality-weighted)
df_hash = hash_index[h].len()
hash_weight = 1.0 / ln(df_hash + 1.8)
score += 3.0 * hash_weight
if df_hash == 1: score += 9.0   // unique hash = massive boost

# Evidence Gate (must pass BEFORE AutoMatched is returned)
primary_evidence = any of:
  - HashOverlap.overlap >= 1
  - AliasStrict matched
  - deep_ratio >= 0.12 OR deep_hits >= 2
  - ini_section_hits + ini_content_hits >= 1

# Tie-break policy
if (best - second) < 1.0 AND both have primary evidence → force NeedsReview
if (best - second) < 0.5 regardless → force NeedsReview

# Score clamping
score = score.clamp(0.0, 100.0)
```

---

### Normalization & Tokenization

```
normalize(s):
  lowercase → replace non-alnum with space → collapse spaces
  strip noise prefixes: [mod], disabled, [skin], etc.

tokens(s):
  split on space → remove stopwords → min_length=4 (allow short_token_whitelist for 2-3 char)
  stopwords: {mod, skin, preset, version, ver, v, fix, shader, tex, texture,
              override, resource, commandlist, key, ini, dds}

INI key blacklist (skip token): run, handling, match_priority, drawindexed, vb, ib, ps, vs, cs, format, stride
INI key whitelist (prefer): texture, resource, filename, path, name, character

Token buckets (separate sets, used for Evidence Gate counters):
  folder_tokens, deep_name_tokens, ini_section_tokens, ini_content_tokens
  observed_tokens = union(all above)
```

---

### Hash Extraction

```
extract_hashes_from_ini_text(text) → Vec<String>:
  accept: "hash = d94c8962", "hash=0xd94c8962", "hash = 00000000d94c8962" (16-hex → last 8)
  output: lowercase 8-char hex
  case-insensitive, ignore invalid tokens
```

---

### FolderSignals Caching

```
FolderSignals {
  folder_tokens, deep_tokens, ini_hashes,
  ini_section_tokens, ini_content_tokens,
  fingerprint: (path, mtime, total_size_bytes, ini_count)
}
→ cache invalidates when fingerprint changes
→ reused across Quick and Full mode runs in the same scan session
```

---

### Candidate Seeding (Prevents O(N) Scoring)

- Seed from `hash_index[h]` for each found hash.
- Seed additional candidates from `token_index` sorted by postings length ascending (rarest first) — union until `seed_cap = 200`.
- If `candidate_count < min_pool = 5`: replenish from rarest observed tokens until `seed_cap`.
- Sort candidates: `score desc → name asc → entry_id asc` (deterministic).

---

### Integration Points

| Component              | Detail                                                                                                                                  |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Master DB / hash_index | `HashMap<hash_str, Vec<entry_id>>` — loaded at startup (Epic 09)                                                                        |
| Token DF index         | `HashMap<token, Vec<entry_id>>`, built at startup from `schema.json` aliases                                                            |
| `hash_db` schema       | Per-entity `hash_db: HashMap<String, Vec<String>>` in `schema.json` (backward-compatible, default `{}`)                                 |
| GameBanana API         | `reqwest::Client` with 3s timeout; public API v11; no auth key required                                                                 |
| AI Provider            | Configured in Settings (Epic 04); API key in OS keychain via `keyring` crate                                                            |
| ScanReviewModal        | Reads `Vec<ScoredCandidate>` from Tauri state; "Commit" triggers Epic 27                                                                |
| Debug logs             | Feature-flag `EMMM2_MATCH_DEBUG=1` prints: mode, best/second scores, margin, primary evidence flags, `foreign_strong_hits`, scan counts |

### Security & Privacy

- **No mod file content sent to external APIs** — only folder names and normalized entity names are in AI prompts.
- **No user PII in AI prompts** — prompts contain only structural tokens and candidate names.
- **GameBanana API calls are triggered only** when a GB URL is found in mod signals — no blanket external calls.
- **AI API keys in OS keychain** (`keyring` crate) — never stored in `settings.json` or sent over IPC.

---

## 4. Dependencies

- **Blocked by**: Epic 25 (Scan Engine — `Vec<ScanResult>` with `FolderSignals` as input), Epic 09 (Object Schema/Master DB — golden corpus, stopwords, `hash_db`, schema.json).
- **Blocks**: Epic 27 (Sync DB — consumes approved `Vec<ScoredCandidate>`), Epic 23 (Mod Import — shared `analyze()` function for per-archive categorization).
