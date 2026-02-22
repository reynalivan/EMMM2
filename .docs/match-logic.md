# Prompt (copy-paste to coding agent)

Role: **Senior Rust Engineer**. Goal: **Replace** the current matching logic in `deep_matcher.rs` with a staged matcher that supports **2 modes**: `Quick` and `FullScoring`, uses **scoring + margin acceptance**, and stops at **L3** (no fuzzy). **AI is optional and OFF by default**, and may run only if still `NeedsReview` after L3.

Key intent (no gray area):

- **Direct Name Match must remain**, but **ONLY as a supporting signal** (never the primary/sole reason to auto-match) to reduce false detections.
- If hashes are missing/incomplete, the **main alternative signal** is **Deep Content Scan (recursive)**: subfolder names + file stems + **INI content tokens**.
- **Skin Alias Match (L0)** must exist and remain strong (strict).

---

## Files / Context

- Refactor target: `src-tauri/src/.../deep_matcher.rs` (existing implementation)
- Use existing folder scanning utilities (e.g. `walker.rs` / `FolderContent`) if present.
- Reuse/extend INI parsing already present (search for hash parsing in `conflict.rs`), but implement the robust extractor and content tokenization below.
- Game schema JSON lives in `src-tauri/resources/schemas/` (example: `gimi.json`). Add placeholder keys as described.

---

## 1) Data model changes (DB + schema placeholders)

### 1.1 DB entry: hash_db (Replaces flat hashes array)

Extend the DB entry struct used by the matcher with:

- `hash_db: HashMap<String, Vec<String>>` (optional in JSON, backward compatible via serde default)
- _Note: This supersedes the older flat `hashes: Vec<String>` array strategy._

Rules:

- If `hash_db` is missing -> `{}`.
- Matcher iterates `.values()` to collect all hashes for the object.
- **No hash present must NOT reduce match ability**; matcher must still work via deep scan.

Intended shape:
Inside a character object like `{"name": "Raiden" ... }`:

```json
"hash_db": {
  "Default": ["d94c8962"],
  "Boss": ["abc12345"]
}
```

Rules:

- App must load even if `hash_db` is absent.
- Matcher must **NOT require** `hash_db` to exist.

### 1.3 Schemas placeholder (resources/schemas)

In each per-game config JSON, add valid placeholders (keep empty for now but must be valid JSON):

- `"stopwords": []`
- `"short_token_whitelist": []`
- `"ini_key_blacklist": []`
- `"ini_key_whitelist": []`

---

## 2) Matcher API & output (explicit)

### 2.1 Enums

- `enum MatchMode { Quick, FullScoring }`
- `enum MatchStatus { AutoMatched, NeedsReview, NoMatch }`
- `enum Confidence { High, Medium, Low }`

### 2.2 Result

`MatchResult` includes:

- `status: MatchStatus`
- `best: Option<Candidate>`
- `candidates_topk: Vec<Candidate>` (default top_k=5)
- `evidence: Evidence`

`Candidate` includes:

- `entry_id` (or stable identifier)
- `name`, `object_type`
- `score: f32`
- `confidence: Confidence`
- `reasons: Vec<Reason>` (structured)

`Evidence` includes:

- `matched_hashes: Vec<String>` (unique, sorted)
- `matched_tokens: Vec<String>` (unique, sorted)
- `matched_sections: Vec<String>` (unique, sorted)
- `scanned_ini_files: usize`
- `scanned_name_items: usize`

`Reason` must be structured enum (minimum):

- `HashOverlap { overlap: u32, unique_overlap: u32 }`
- `AliasStrict { alias: String }`
- `DirectNameSupport { token: String }` // L1 supporting only
- `TokenOverlap { ratio: f32 }`
- `DeepNameToken { token: String }`
- `IniSectionToken { token: String }`
- `IniContentToken { token: String }`
- `AiRerank { ai_score: f32 }`
- `MechanicalRerank { reason: String }` (e.g. GB exact match, keyword overlap)

Deterministic ordering:

- sort candidates by `score desc`, then `name asc`, then `entry_id asc`.

---

## 3) Normalization & tokens (explicit)

Normalization `normalize(s)`:

- lowercase
- replace non-alphanumeric with spaces
- collapse spaces
- strip known noise prefixes in folder name: `[mod]`, `disabled`, `[skin]`, etc.

Tokenization `tokens(s)`:

- split spaces
- remove stopwords: `mod, skin, preset, version, ver, v, fix, shader, tex, texture, override, resource, commandlist, key, ini, dds`
- min length = 4
- allow **short-token whitelist** (2–3 chars) (default empty; configurable)

Important:

- **No raw substring contains**. All matching uses token/boundary semantics.

---

## 4) Hash extraction (must)

Implement `extract_hashes_from_ini_text(text) -> Vec<String>`: Accept:

- `hash = d94c8962`
- `hash=0xd94c8962`
- `hash = 00000000d94c8962` (16 hex) -> take **last 8**

Rules:

- accept hex case-insensitive
- output lowercase 8-hex
- ignore invalid tokens

Hash sources:

- Prefer `.ini` files from existing walker (`FolderContent.ini_files`).
- If `content` missing, do bounded scan for `.ini`.

---

## 5) Deep Content Scan (L3) — PRIMARY fallback when hashes incomplete

**Important:** A mod folder may contain **multiple** `**.ini**` **files**. The matcher must support this by scanning **up to the mode budget**, aggregating hashes/tokens across all scanned `.ini` files, and remaining deterministic (stable file ordering + stable caps). Deep scan must extract **tokens from**:

1.  **Subfolder names** (recursive within depth)
2.  **File stems** (filename without extension; recursive within depth)
3.  **INI content tokens** (budgeted):
    - **Section headers**: lines like `[TextureOverrideDilucVB]`
      - extract inside brackets, split PascalCase + non-alnum
      - strip common prefixes/tokens: `TextureOverride, ShaderOverride, Resource, CommandList, Key, Present, Draw`

    - **Key names**: left side of `=` within INI (split into tokens)
    - **String values** that look like file paths/names:
      - if RHS contains `.dds`/`.png`/`.jpg`/`.ini`/`.buf`/`.txt`, tokenize path segments + stem

    - Ignore purely numeric tokens and stopwords.

Budget rules (must enforce):

- a folder can have many `.ini`; scan **multiple files** up to budget, not just one
- always choose `.ini` files deterministically (sort by relative path asc), and scan in that order
- never read binaries; only `.ini` text
- never read binaries; only `.ini` text
- cap bytes read per ini, and cap total bytes per match (see mode budgets)

---

## 6) Scoring + margin acceptance (shared; no ambiguity)

Maintain `ScoreState` per entry:

- `score: f32`
- `reasons: Vec<Reason>`
- `overlap: u32`, `unique_overlap: u32`

Stage evaluation:

- If 0 candidates -> continue.
- If 1 candidate -> `second_score = 0.0`.
- Accept iff: `best_score >= threshold_stage` AND `(best_score - second_score) >= margin_stage`.

**Hard rule: Direct Name Match (L1) can NEVER auto-match by itself.** Implementation: L1 is never an acceptance stage; it only adds score.

**Evidence gate (prevents false detection):** even if score+margin pass, do **not** return `AutoMatched` unless the best candidate has at least **one Primary Evidence**.

Primary Evidence is any of:

- Hash evidence: `HashOverlap.overlap >= 1`
- Alias evidence: `AliasStrict` matched
- Deep structure evidence: (`deep_ratio >= 0.12`) OR (`deep_hits >= 2`)
- INI evidence: (`ini_section_hits + ini_content_hits >= 1`)

Definitions:

- `deep_hits` = count of matched tokens from the **deep name token** intersection (subfolders + file stems).
- `deep_ratio` = `deep_hits / max(deep_tokens_count, 1)`.
- `ini_section_hits` = count of matched tokens originating from section-header tokenization.
- `ini_content_hits` = count of matched tokens originating from key-name + RHS path-like tokenization.

L1 direct-name hits (`DirectNameSupport`) are **not** primary evidence.

Final decision after last L3 stage:

- if `best_score >= review_min_score` -> `NeedsReview` (return top-k)
- else -> `NoMatch`

---

## 7) Signals & weights (tuned to reduce false positives)

### Candidate seeding (prevents O(N) scoring)

At each stage, if the candidate set is empty, seed candidates using indexes:

- From hashes: union of `hash_index[h]`
- From tokens (folder/deep/ini tokens): for each token `t` present in `token_index`, add its entry_ids.
- Cap seeded candidates to a reasonable max (e.g. 200) by preferring rarer tokens (lower DF / smaller postings list).

### Hash scoring (strong but optional; quality-weighted)

For each found hash `h`:

- `df_hash = hash_index[h].len()`
- `hash_weight(h) = 1.0 / ln(df_hash + 1.8)`
- candidates = `hash_index[h]`
- for each candidate entry:
  - `overlap += 1`
  - if `df_hash == 1` then `unique_overlap += 1`
  - score add:
    - `score += 3.0 * hash_weight(h)`
    - if `df_hash == 1` also add `+9.0`

- add `Reason::HashOverlap { overlap, unique_overlap }` once per candidate at stage end.

Notes:

- Hashes are **not required**. If none found or none map to DB, matching continues via Deep Content Scan.

### Skin Alias strict (strong, precise)

- L0: each entry alias is a token sequence.
- Alias match if **all alias tokens** exist in tokenset:
  - Quick: folder tokens only
  - Full: folder tokens + deep/ini tokens (after deep scan stage, re-check aliases if needed)

- score `+12` and add `Reason::AliasStrict`.

### L2 Token overlap (medium)

- Quick uses simple ratio:
  - `ratio = |intersection(folder_tokens, entry_tokens)| / max(|folder_tokens|,1)`

- Full uses weighted IDF-lite ratio (recommended):
  - `w(t)=ln((N+1)/(df(t)+1))+1`
  - `ratio = sum(w in intersection)/max(sum(w in folder_tokens), eps)`

- score `+12 * ratio` and add `Reason::TokenOverlap{ratio}`.

### L3 Deep scan (PRIMARY fallback when hashes incomplete)

Compute deep token ratio:

- `deep_ratio = |intersection(deep_tokens, entry_tokens)| / max(|deep_tokens|,1)`
- score `+16 * deep_ratio`
- plus small boosts per matched deep token (cap total boost)
- add representative `Reason::DeepNameToken`.

INI evidence (section + content tokens):

- `ini_ratio = |intersection(ini_tokens, entry_tokens)| / max(|ini_tokens|,1)`
- score `+8 * ini_ratio`
- add representative `Reason::IniSectionToken` / `Reason::IniContentToken`.

### L1 Direct Name Support (supporting only)

Purpose: help ranking when signals are sparse, but never be the primary reason for AutoMatched.

Token-aware only (boundary semantics):

- If entry `name` token appears in folder tokens => add name-support points
- If entry `tags` token appears in folder tokens => add tag-support points

Weights and caps:

- **Quick:**
  - name token hit: `+4` each, cap total L1 contribution at `10`
  - tag token hit: `+2` each, cap total tag contribution at `6`

- **FullScoring:**
  - name token hit: `+2` each, cap total L1 contribution at `6`
  - tag token hit: `+1` each, cap total tag contribution at `4`

Add `Reason::DirectNameSupport{token}` for representative hits (cap reasons per candidate). Hard rule: L1 never has its own acceptance stage; it only adjusts scores.

## 8) Two modes: stage-by-stage flow + budgets (final)

### Mode QUICK (fast)

Budgets:

- INI scan: max 2 `.ini` files, **root folder only**, max 256KB per file.
- Deep scan names: depth=1; cap `scanned_name_items <= 150`.

Stages:

1.  **Q1 Hash Sniff**
    - parse hashes from root `.ini` only.
    - seed candidates from hash_index.
    - apply hash scoring.
    - accept with `T_hash_quick`, `M_hash_quick` (also must satisfy Evidence Gate).

2.  **Q2 Skin Alias strict**
    - folder tokens only.
    - accept with `T_alias_quick`, `M_alias_quick` (also Evidence Gate is satisfied by Alias).

3.  **Q3 L3-lite Deep Scan** (PRIMARY fallback)
    - collect tokens from immediate subfolders + file stems (depth=1).
    - parse root INI (budgeted):
      - section header tokens
      - key-name tokens (LHS)
      - path-like RHS stems/segments (only when RHS looks like a file/path)

    - apply deep scoring.
    - accept with `T_deep_quick`, `M_deep_quick` (must satisfy Evidence Gate).

4.  **Q4 L2 Token overlap**
    - folder tokens vs entry tokens.
    - accept with `T_token_quick`, `M_token_quick`.

5.  **Q5 L1 Direct Name Support**
    - apply small supportive boosts.
    - NO dedicated acceptance here; only affects final ranking.

End:

- if accepted earlier => AutoMatched
- else if `best_score >= review_min_score_quick` => NeedsReview
- else NoMatch

### Mode FULLSCORING (accurate, still L0–L3)

Budgets:

- INI scan: max 10 `.ini` files total, recursion depth=3, max total bytes=1MB.
- Deep scan names: depth=3; cap `scanned_name_items <= 500`.

Stages:

1.  **F1 Hash Scan (budgeted)**
    - parse hashes from ini files within depth/budget.
    - seed candidates from hash_index.
    - apply hash scoring.
    - accept with `T_hash_full`, `M_hash_full`.

2.  **F2 Skin Alias strict (early)**
    - run alias strict using **folder tokens only**.
    - accept with `T_alias_full`, `M_alias_full`.
    - Note: alias strict will be re-checked again after deep/ini tokens are collected (see Enhancements).

3.  **F3 L3 Deep Content Scan (PRIMARY fallback)**
    - recursive tokens from subfolders + file stems.
    - parse INI (budgeted) extracting:
      - section tokens
      - key-name tokens (LHS of `=`)
      - path-like RHS tokens (filenames/stems + path segments)
      - ignore numeric-only and stopwords

    - apply deep scoring.
    - accept with `T_deep_full`, `M_deep_full` (also Evidence Gate).

4.  **F4 L2 Weighted token overlap**
    - optional IDF-lite (recommended): `w(t)=ln((N+1)/(df(t)+1))+1`
    - ratio = `sum(w in intersection)/max(sum(w in folder_tokens), eps)`
    - add `+12*ratio`
    - accept with `T_token_full`, `M_token_full`.

5.  **F5 L1 Direct Name Support**
    - apply small supportive boosts.
    - NO dedicated acceptance.

End:

- if accepted => AutoMatched
- else if `best_score >= review_min_score_full` => NeedsReview
- else NoMatch

---

## 9) Default thresholds (explicit; configurable)

Define in `MatchOptions` (defaults):

- `top_k = 5`

Quick:

- `T_hash_quick = 10.0`, `M_hash_quick = 6.0`
- `T_alias_quick = 12.0`, `M_alias_quick = 6.0`
- `T_deep_quick = 14.0`, `M_deep_quick = 4.0`
- `T_token_quick = 12.0`, `M_token_quick = 4.0`
- `review_min_score_quick = 10.0`

Full:

- `T_hash_full = 10.0`, `M_hash_full = 4.0`
- `T_alias_full = 12.0`, `M_alias_full = 4.0`
- `T_deep_full = 16.0`, `M_deep_full = 3.0`
- `T_token_full = 14.0`, `M_token_full = 3.0`
- `review_min_score_full = 12.0`

Confidence mapping:

- accepted in Hash or Alias stage -> High
- accepted in Deep stage -> Medium
- accepted in Token stage -> Medium
- NeedsReview -> Low

---

## 10) AI stage (optional OFF)

- `opts.ai_enabled` default **false**.
- Run AI only if status would be `NeedsReview` after completing L3.
- Input: compact JSON summary of signals (folder tokens, deep tokens sample, hashes, section tokens, top-k candidates with reasons).
- Output: `{ entry_id: score_0_1 }`.
- Accept only if `ai_best >= 0.7` AND `(ai_best - ai_second) >= 0.15`.
- Cache by `(signals_hash, db_version)`.

---

## 11) Edge cases (must)

- Folder can contain **0..many** `**.ini**` **files**; always handle gracefully.
- `.ini` can be large or numerous; enforce budgets and continue with partial signals.
- No `.ini` -> hash stage yields none; continue deep scan.
- Hashes found but no DB match -> continue deep scan; do not early NoMatch.
- Ties / low margin -> NeedsReview.
- Any IO budget exceeded -> stop scanning more; proceed with what’s collected.

---

## 12) Deliverables

1.  Updated `deep_matcher.rs` with the replacement logic.
2.  Index builder updates for `hash_index` and token DF (if used).
3.  Add schema placeholder `hash_db: {}` into `resources/schemas/*.json` safely.
4.  Unit tests:
    - unique hash => AutoMatched
    - 16-hex => last 8 extracted
    - deep scan works when hashes missing => can AutoMatch/NeedsReview based on margin
    - direct name alone cannot AutoMatch
    - margin not met => NeedsReview
    - ai_enabled=false => AI never called

5.  Debug logs (feature-flag) printing stage summary and top2 scores + margin.

---

## 13) Additional polish (must implement)

These remove remaining gray areas and real-world failure modes.

### 13.1 Candidate pool replenishment (avoid wrong small pool)

Besides “seed only when empty”, also replenish when pool is too small:

- If `candidate_count < min_pool` (default 5), seed additional candidates from the **rarest** observed tokens (folder/deep/ini), using postings length ascending, until `seed_cap`.

### 13.2 Observed token buckets (required for Evidence Gate)

Maintain observed tokens as **separate sets** (deduped):

- `folder_tokens`
- `deep_name_tokens`
- `ini_section_tokens`
- `ini_content_tokens` And a derived union:
- `observed_tokens = union(all above)`

Evidence Gate counters must be computed per bucket:

- `deep_hits` from `deep_name_tokens ∩ entry_tokens`
- `ini_section_hits` from `ini_section_tokens ∩ entry_tokens`
- `ini_content_hits` from `ini_content_tokens ∩ entry_tokens`

### 13.3 Deterministic capping (tokens + files)

When applying caps (tokens, reasons, files):

- Sort tokens by **rarity first** (lower DF/postings), then alphabetically.
- For `.ini` selection: sort file paths (relative) ascending, then take first N. This ensures stable results across runs.

### 13.4 Windows symlink/junction safety

Folder walker must not infinite-loop on symlinks/junctions:

- Do not follow symlinks by default, OR
- Track visited canonical paths and skip repeats.

### 13.5 INI decoding fallback

When reading `.ini`:

- Try UTF-8.
- If UTF-8 fails, fallback to lossy decoding (or detect UTF-16 LE BOM and decode).
- Never crash on decode errors; treat undecodable bytes as skipped/ignored.

### 13.6 INI include/reference tokening (no extra IO)

If RHS looks like an `.ini` path (contains `.ini`), do **not** read it in Quick.

- Still extract tokens from the path stem/segments as `ini_content_tokens`. (FullScoring may read it only if it is already within selected `.ini` files by normal walk/budget; no special extra IO.)

### 13.7 Object-type gating (soft)

If the matcher has context of desired `object_type` (Character/Weapon/UI):

- Apply a small penalty for mismatched types (e.g. `-2.0`) rather than filtering hard.
- If no context available, skip this.

### 13.8 Pack / multi-entity heuristic

Detect mod packs with mixed strong signals:

- If `>=2` candidates have Primary Evidence and both scores are above `review_min_score`, force `NeedsReview` even if margin passes. This prevents auto-matching a pack folder to the wrong single entry.

### 13.9 Golden corpus test hook (quality)

Add a tiny “golden corpus” test harness:

- A folder of test fixtures (names + small `.ini` snippets) with expected outcome:
  - expected `status` and expected `best.entry_id` OR expected `NeedsReview`. This enables safe threshold tuning.

## 7) 10-Stage Pipeline (Full Scoring)

To effectively match varying signal strengths while ensuring zero false positives, the FullScoring pipeline implements **10 distinct stages**:

1.  **Hash Scan:** Exact MD5/Blake3 overlaps. Highest priority.
2.  **Alias Strict:** Requires all alias words to be present.
3.  **F3A Substring Pass A:** Substring matching over file stems and subfolder names. Good for "zibai lunar qilin" matching "zibai".
4.  **Deep Token Overlap (Skipped):** Bypassed architecture in favor of F4.
5.  **F3B Substring Pass B:** Substring matching over INI-derived strings (section headers, path-like values).
6.  **Alias Re-check:** Re-apply strict alias after deep INI token expansion.
7.  **Weighted Token Overlap:** Apply IDF-aware (Term Frequency - Inverse Document Frequency) token overlap scoring.
8.  **Direct-Name Support:** Adds support points, cannot auto-match alone.
9.  **AI Rerank (Optional):** Trait-based LLM reranking for `NeedsReview` candidates. OFF by default.
10. **Mechanical Rerank (GameBanana Supplement):** Fast point-based rerank. Extracts metadata from the **GameBanana API v11** to provide exact-match bonuses and punishing mismatch penalties (detailed below).
11. **F9 Root Folder Rescue:** Last-resort fallback. Runs purely on the normalized root folder name if the status remains `NoMatch`.

## 8) Mechanical Rerank & GameBanana Enrichment

When a candidate finishes the primary 8 stages but remains in `NeedsReview`, it enters the **Mechanical Rerank** stage.
This stage actively incorporates **GameBanana API v11** data:

- **Mod Name Exact Match:** If the API's `mod_name` directly matches `DbEntry`'s name or alias, it awards a massive `+25.0` point bonus.
- **Root Category Validation:** Implements a strict penalty (`-15.0`) if the GameBanana API reports the category as "Skins" but the candidate object type is "Weapon", "UI", etc.
- **Description Keyword Overlap:** Extracts words >3 chars from the GB description, stripping HTML. Awards up to `+6.0` points for overlap with local tags/name.
- **File Stem Validation:** Awards structural points for partial text matching against the `_aFiles` extracted from the API.

## 9) Ambiguity controls

- To prevent false auto-acceptance, define `top1` and `top2`.
- if `(top1.score - top2.score) < Margin`, downgrade `AutoMatched` to `NeedsReview` (except when the score is an absolute massive outlier, e.g. perfect hash match).
- The review UI will display `candidates_topk` so the user decides.

## 10) Output

At the end of parsing the folder, the function must return the exact structured `MatchResult` so that the frontend review table receives predictable data.

### 13.10 Threshold calibration logs

Under a debug feature flag, add a structured log dump per match:

- mode, best/second scores, margin, primary evidence flags, foreign_strong_hits, scanned counts. This is required to tune thresholds without guessing.

## Enhancements (apply all)

Implement these improvements as part of the refactor.

### Negative evidence (conflict penalty)

Purpose: reduce false auto-matches when a folder contains signals for multiple entries.

Definitions:

- `strong_token`:
  - FullScoring: `df(token) <= max(3, N/200)`
  - Quick: token length >= 5 AND postings length `<= 2` (from token_index)

Rules:

1.  For each candidate A, compute `foreign_strong_hits`:
    - For every strong token `t` present in the **observed token set** (folder + deep + ini tokens):
      - if `t` is NOT in A’s token set
      - AND `token_index[t]` contains some entry id other than A
      - then count as 1 foreign hit.

2.  Apply penalty:
    - FullScoring: `score -= 2.5 * foreign_strong_hits` (cap 10)
    - Quick: `score -= 1.5 * foreign_strong_hits` (cap 8)

3.  Ambiguity override:
    - If top2 both have Primary Evidence AND `best_score - second_score < margin_stage`, force `NeedsReview` (do not AutoMatch).

Add enum reason:

- `Reason::NegativeEvidence{ foreign_strong_hits: u32 }`
- `Reason::MechanicalRerank`
- `Reason::GameBananaEnrichment`

### Distinctiveness-aware candidate seeding

When seeding candidates from token_index:

- sort tokens by postings length ascending (rarer first)
- union postings until reaching `seed_cap` (default 200)
- if still empty, allow fallback seeding from deep/ini tokens.

### Per-game stopwords + short-token whitelist

Make stopwords + short-token whitelist configurable per game:

- Load from schema if present: `stopwords: []`, `short_token_whitelist: []`.
- If absent, use safe defaults.

### INI parsing (semi-structural) + content tokens

Do not tokenize raw lines blindly. Parse:

- section headers: `[SectionName]`
- key/value lines: `key = value` Extract tokens from:
- section name (PascalCase split)
- key name (LHS)
- RHS path-like stems/segments only (filenames and folder segments) Ignore:
- numeric-only tokens
- stopwords

Add enum reason:

- `Reason::IniContentToken{ token: String }`

### INI key blacklist/whitelist

Reduce noise:

- default blacklist keys: `run, handling, match_priority, drawindexed, vb, ib, ps, vs, cs, format, stride`
- default whitelist keys: `texture, resource, filename, path, name, character` Rules:
- if key is blacklisted -> skip key tokens
- if whitelist non-empty -> only accept key tokens if key in whitelist

### Hash quality weighting

Already included in **Hash scoring (quality-weighted)** above. Do not implement twice.

### Alias strict re-check after deep/ini

FullScoring must run alias strict twice:

- early: folder tokens
- late: after deep/ini tokens collected Late alias hit may satisfy Evidence Gate.

### Explainability summary (one-liner)

Add optional `summary: String` on MatchResult (or computed helper) that compresses the reason:

- AutoMatched: “Matched by \+ ”
- NeedsReview: “Ambiguous: vs ” Deterministic and short.

### Cache folder signals

Cache extracted signals per folder to avoid repeated IO:

- `FolderSignals { folder_tokens, deep_tokens, ini_hashes, ini_section_tokens, ini_content_tokens, fingerprint }`
- fingerprint = (path, mtime, total size, ini count) or fast hash of these.
- reuse cache in both modes; invalidate on fingerprint change.

### Tests to add

- negative evidence penalizes and forces NeedsReview on mixed-signal
- alias re-check after deep can rescue match

### Score clamping + reason caps (polish)

To keep output stable and UI/logs manageable:

- Clamp per-candidate `score` into a bounded range after each stage: `score = score.clamp(0.0, 100.0)`.
- Cap `reasons` per candidate (deterministic):
  - `max_reasons_per_candidate = 12`.
  - Prefer keeping: `HashOverlap`, `AliasStrict`, `NegativeEvidence`, then the first N token-based reasons in stable order.

- Cap stored evidence token lists in `Evidence` (deterministic):
  - `matched_tokens` max 50, `matched_sections` max 50, `matched_hashes` max 50.

### Tie-break & ultra-close policy (polish)

To avoid accidental auto-matches when candidates are extremely close:

- After applying score+margin acceptance **and** Evidence Gate, apply an additional final check:
  - If `best_score - second_score < 1.0` AND (top2 both have Primary Evidence) => force `NeedsReview`.

- If `best_score - second_score < 0.5` regardless of primary evidence => force `NeedsReview`.
- Keep deterministic ordering for stable UI.

Add tests:

- ultra-close top2 (<1.0) with primary evidence => NeedsReview
- ultra-close (<0.5) => NeedsReview
