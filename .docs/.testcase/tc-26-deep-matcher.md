# Test Cases: Deep Matcher & Auto-Categorization (Epic 26)

## A. Requirement Summary

- **Feature Goal:** A staged matching engine to auto-categorize scanned mod folders without human intervention (zero false positives over pure accuracy) using Hashes, Aliases, Tokens, optional LLM fallback, and GameBanana API enrichment.
- **User Roles:** System (Automated Scanning/Categorization), User (Manual Review).
- **Success Criteria:**
 - Quick mode (<50ms per mod) utilizing early hash/alias match.
 - ≥70% Quick mode accuracy, ≥85% Full mode accuracy (200-mod benchmark).
 - 0 false positives committed silently. Review gate enforces user oversight for unsure matches.
 - Strict evidence requirements (Primary Evidence) before`AutoMatched`.
 - Negative evidence penalties apply to prevent crossover corruption.
 - NEW (Phase 5): "Review Prefix Normalization" strips meaningless tags like`[V1.2]` or`(Fixed)` from the generated user-facing Mod Name during import.

## B. Coverage Matrix

**Requirement File**:`e:\Dev\EMMM2NEW\.docs\requirements\req-26-deep-matcher.md`

| Acceptance Criteria | Covered by TC IDs |
| :--------------------------------- | :---------------- |
| AC-26.1.1 (Alias Strict) | TC-26-001 |
| AC-26.1.2 (Token Overlap) | TC-26-002 |
| AC-26.1.3 (Hash Overlap) | TC-26-003 |
| AC-26.1.4 (DirectName Supp) | TC-26-004 |
| AC-26.1.5 (No Signals) | TC-26-005 |
| AC-26.1.6, AC-26.3.5 (Tiebreaker) | TC-26-006 |
| AC-26.2.1, AC-26.2.2 (Deep Token) | TC-26-007 |
| AC-26.2.4 (Stopword Skip) | TC-26-008 |
| AC-26.2.5 (Malformed INI) | TC-26-009 |
| AC-26.3.3 (Evidence Gate Block) | TC-26-010 |
| AC-26.3.1 (Evidence Gate Pass) | TC-26-011 |
| AC-26.4.1 (Negative Penalty) | TC-26-012 |
| AC-26.4.3 (Clean Folder) | TC-26-013 |
| AC-26.5.1, AC-26.5.2 (GameBanana) | TC-26-014 |
| AC-26.5.4 (GameBanana Timeout) | TC-26-015 |
| AC-26.6.1, AC-26.6.2 (AI Tiebreak) | TC-26-016 |
| AC-26.6.4 (AI JSON Failure) | TC-26-017 |
| AC-26.6.5 (AI Cache Hit) | TC-26-018 |
| AC-26.7.1 (Review Modal Load) | TC-26-019 |
| AC-26.7.2 (Override Mapping) | TC-26-020 |
| AC-26.7.3 (Review Cancel) | TC-26-021 |
| Phase 5: Prefix Normalization | TC-26-022 |

## C. Test Cases

| TC ID | Scenario | Type | Priority | Failure Severity | Preconditions | Test Data | Steps | Expected Result | Coverage |
| :-------- | :-------------------------------------- | :------- | :------- | :--------------- | :------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------ | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------- |
| TC-26-001 | Alias Strict Match (Q2) | Positive | High | S1 | Master DB contains alias`Albedo`. | Folder strictly named`Albedo`. | 1. Pass folder to Quick Mode pipeline. | Matcher returns`AutoMatched`, Confidence`High`, target folder`Albedo`. Fast mode returns instantly without checking deep.ini files. | AC-26.1.1 |
| TC-26-002 | Token Overlap Match (Noisy folder, Q4) | Positive | High | S1 | Master DB has object`Hu Tao`. | Folder named`[V1.2] Cool_Hu_Tao_Mod by Author`. | 1. Pass to Matcher.<br>2. Pipeline runs stopword filtering. | Stopwords`cool`,`mod`,`by`,`author` drop. Token`hutao` meets scoring threshold vs Master DB`Hu Tao`. Returns`AutoMatched`. | AC-26.1.2 |
| TC-26-003 | Hash Snipe Match (Q1) | Positive | High | S1 | Active`hash_index.json` has`hash=0xd94c8962` mapped to`Diluc`. | INI file contains known`hash=0xd94c8962`. | 1. Pass INI content through Quick Mode Hash check. |`hash_index` resolves. Immediate`AutoMatched`, Confidence`High`, tagged to Diluc Object. | AC-26.1.3 |
| TC-26-004 | DirectNameSupport is Insufficient Alone | Negative | Medium | S2 | Folder has no INI hashes, just a vague name. | Folder`DilucMod` (only L1 hit). | 1. Pass to Matcher. | DirectName alone does not provide sufficient "Primary Evidence". Scoring proceeds but caps at`NeedsReview`. It never evaluates as`AutoMatched` strictly based solely on a folder name. | AC-26.1.4, AC-26.3.4 |
| TC-26-005 | No Signals found | Negative | High | S1 | Junk folder name, no INI files, no hashes. | Folder named`asd123_xyz`. | 1. Pass to Full Scoring pipeline. | Zero signal stages pass thresholds. Result evaluates definitively as`NoMatch` and is placed into the "Uncategorized" bucket. | AC-26.1.5 |
| TC-26-006 | Tiebreaker Forcing NeedsReview | Edge | High | S1 | Mod pack blends multiple characters. | Pack containing tokens for both`Raiden` and`Ganyu`. | 1. Evaluate candidate scores.<br>2. Ensure score difference is < 1.0. | Despite passing the minimum score limit, the margin between the #1 and #2 candidate is too slim. Relegated to`NeedsReview` explicitly to prevent wrong assignments. | AC-26.1.6, AC-26.3.5 |
| TC-26-007 | Deep Content Extraction | Positive | High | S1 | Vague root folder name, but explicit internal structure. | Mod contains subfolder`Diluc_Weapon` and`TextureOverrideDilucVB.ini`. No base global hashes. | 1. L3 Deep Scan extracts section structures.<br>2. Tokenizes folder and INI header names. |`Diluc` is resolved from internal deep tokens. Reaches Primary Evidence threshold via deep counts. Matches Diluc as`AutoMatched`. | AC-26.2.1, AC-26.2.2 |
| TC-26-008 | Deep Content Skip Stopwords | Positive | Low | S3 | INI has standard migoto keys. | INI with keys`run`,`drawindexed`,`texture`. | 1. Run L3 Deep Scan extraction phase. | Extractor entirely discards blacklisted noise/stopwords (`run`,`texture`), preventing false-positive token accumulation. | AC-26.2.4 |
| TC-26-009 | Malformed INI Parsing Resilience | Negative | Medium | S2 | INI file is corrupted binary data. |`.ini` file is actually a renamed`.exe`. | 1. Run pipeline. | Matcher attempts fallback decoding, catches parsing failure, logs event, and skips the file gracefully without crashing the Rust panic handler. | AC-26.2.5 |
| TC-26-010 | Evidence Gate Blocked | Negative | High | S1 | Folder achieves high score, but only through generic non-character tokens. | Generic token spam folder. | 1. Check matching candidate rules engine. |`AutoMatched` explicitly denied due to lacking Primary Evidence (Hash, Alias, or specific Model match). Returns`NeedsReview`. | AC-26.3.3 |
| TC-26-011 | Evidence Gate Passed | Positive | High | S1 | Folder passes minimum score. | Folder matches score + contains >= 1 known hash hit. | 1. Evaluate final candidate. | Security gate verifies criteria > 1 overlap hash, passes safely returning`AutoMatched`. | AC-26.3.1 |
| TC-26-012 | Negative Evidence Penalty Cap | Positive | Medium | S1 | Mod combines two specific characters internally. | Folder hits`Raiden` but contains distinctly foreign`Ganyu` tokens in deep struct. | 1. Pipeline reaches F6 rule eval. | Evaluator detects foreign conflicting strong tokens. Applies`-2.5` multiplier reduction capped structurally at max 10pts down. Drops confidence if needed. | AC-26.4.1 |
| TC-26-013 | Zero Negative Evidence Penalty | Positive | Low | S4 | Pure single-character mod. | Clean folder targeting strictly`Raiden`. | 1. Pipeline reaches F6 rule eval. | 0 foreign hits. Mathematical execution removes 0 points. | AC-26.4.3 |
| TC-26-014 | GameBanana Enrichment Exact Map | Positive | High | S1 | Mod`info.json` contains valid GameBanana URL. |`FolderSignals` contains valid GB URL for "Raiden Skin". | 1. Pipeline accesses GameBanana API endpoint. | API resolves Object. Confers massive +25.0 mathematical point bonus towards candidate "Raiden". | AC-26.5.1, AC-26.5.2 |
| TC-26-015 | GameBanana Timeout Safely Managed | Negative | Medium | S1 | GB API is down or throttled. | Mock Network throttle > 3 seconds. | 1. Trigger pipeline with valid GB URL. | Matcher hits strict 3s timeout cap. Abandons request, logs incident, and defaults back to mechanical folder scoring without freezing imports. | AC-26.5.4 |
| TC-26-016 | AI Reranker Tie Break Resolving | Positive | High | S1 | AI enabled in settings. Ambiguous mod tie requires tiebreak. |`ai_enabled = true`. Ambiguous Mod tie between`Keqing` and`Ganyu`. | 1. AI Reranker stage invoked.<br>2. LLM responds with JSON prioritizing Keqing > 0.70 confidence. | Matcher upgrades status mechanically out of`NeedsReview` into`AutoMatched` explicitly for Keqing due to AI confidence. | AC-26.6.1, AC-26.6.2 |
| TC-26-017 | AI JSON Failure Resilience | Negative | Medium | S2 | AI endpoint offline or returns malformed text instead of JSON. |`opts.ai_enabled = true`. Trigger Mock endpoint to return`500 Server Error` or hallucination. | 1. AI Reranker invoked. | Graceful parsing failure. Retains standard Mechanical`NeedsReview` status automatically without panicking or modifying the DB state improperly. | AC-26.6.4 |
| TC-26-018 | AI Cache Hit Bypassing API | Positive | Low | S4 | Exact same ambiguous mod folder scanned multiple times. | Scan identical struct twice. | 1. Scan first run.<br>2. Clear results and Rescan. | Second scan detects checksum in cache. Immediately uses previous AI response matrix. 0 external API network calls made. | AC-26.6.5 |
| TC-26-019 | Scan Review Modal Load Formatting | Positive | High | S1 | Multi-mod import triggers review UI. | Scan processes 10 batch mods into Tauri memory. | 1. Open UI Scan Review Modal. | State loaded. Table sorts by absolute confidence levels lowest-first. Displays`{name, proposed, reason}` formatted. | AC-26.7.1 |
| TC-26-020 | Target Object User Override | Positive | High | S1 | User is inside the Review Modal. |`NeedsReview` identifies folder as "Hu Tao". | 1. Open Target Object Dropdown.<br>2. Select "Diluc".<br>3. Commit. | Final payload physically assigns DB`parent_id` manually to Diluc, overriding pipeline scoring. | AC-26.7.2 |
| TC-26-021 | Scan Review Cancellation | Negative | Medium | S1 | User is inside Review Modal with pending matches. | Review Modal is active with uncommitted matches. | 1. Click "Cancel Scan".<br>2. Review DB state. | Memory mapping drops instantly. Zero rogue matches are committed to primary SQLite DB accidentally. The imported folders remain "Uncategorized", safe. | AC-26.7.3 |
| TC-26-022 | Review Prefix Normalization | Positive | High | S2 | Mod import has a cluttered name. | Archive named`[V1.2] (New) Awesome Keqing Mod!!.zip`. | 1. Import Archive.<br>2. Check mapped name output in the DB. | The Engine strips the meaningless prefixes and normalizes the folder name to`Awesome Keqing Mod` before DB insertion. | Phase 5 |

## D. Missing / Implied Test Areas

- **[Implied] Performance Profiling**: How does memory scale when 1,000 deep INI files are loaded in Full Scoring? Ensure no Out-of-Memory limits are hit.
- **[Implied] Cancelled Scan Interaction**: What happens if the user cancels the modal while the AI reranker API is currently fetching? UI needs atomic abort checks.

## E. Test Environment Setup

- **Master DB**: Seed with objects "Albedo", "Hu Tao", "Diluc", "Raiden", "Ganyu". Add corresponding`hash_index` mapping for a known`Diluc` hash.
- **AI Mock**: Configure the local setup to route AI Rerank calls to a deterministically mocked LLM stub returning expected JSON. Configure GameBanana endpoint intercept to mock API responses.

## F. Cross-Epic E2E Scenarios

- **E2E-26-001 (End-to-End Scan & Categorize)**: User imports a batch of 50 Mod archives (Epic 23). The Scan Engine (Epic 25) unpacks and passes the extracted`FolderSignals` to the Deep Matcher. The Matcher assigns 45 to`AutoMatched` based on structural deep tokens and hashes. 5 are flagged`NeedsReview`. The User Interface presents the Review Gateway to the user (Epic 26). The user confirms the mappings via the Table overriding 1. Prefix Normalization cleans up the final names. The changes are atomically flushed to SQLite securely updating ObjectList Counts natively without full application refresh.
- **E2E-26-002 (AI Reranker API Integration)**: User enables `AI Matcher`. A deeply obfuscated Mod is processed, bypassing standard Alias/Hash identification blocks (Epic 26). The LLM intercept analyzes the folder structure and returns a high-confidence JSON payload. The Matcher consumes this payload and promotes the finding to the database.
