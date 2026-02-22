# Deep Matcher Pipeline Logic

## 1. Normalization (Pre-processing)
Consistency is key. Apply this before ANY comparison.

-   **Case:** Convert to lowercase.
-   **Symbols:** Replace `_`, `-`, `.`, and `[]` with **Efficiency Space** (` `).
-   **Encoding:** Convert non-ASCII to closest ASCII (Unidecode).
    -   *Ex:* `Raiden_Shogun_[Mod]` -> `raiden shogun mod`

## 2. Layer Logic

### L1: Name Match (Exact/Contains)
**Goal:** Instant match for well-named folders.
-   **Algorithm:** `db_name.contains(folder_name)` OR `folder_name.contains(db_name)`.
-   **Weight:** **100** (Perfect).

### L2: Token Match (Heuristic)
**Goal:** Handle scrambled words (e.g., "Mod Raiden v2").
-   **Input:** Split Folder Name & DB Name into `HashSet<String>`.
-   **Algorithm:** Jaccard Index (Intersection / Union).
-   **Weight:** **Score * 100**.

### L3: Content Scan (Deep Inspection)
**Goal:** Identify generic folders ("Mod 1", "New Folder").
**Constraint:** ONLY run if L1 & L2 < 50.
-   **Action:** Walk folder (max depth 3).
-   **Target:** `*.ini`, `*.ib`.
-   **Logic:** Read `[Constants]` section or Filename.
-   **Weight:** **90** if specific unique asset found.

### L4: Fuzzy Match (Typo Tolerance)
**Goal:** Catch "Raiden Shgum".
-   **Algorithm:** Levenshtein Distance (using crate `strsim`).
-   **Threshold:** Must be > **0.85** normalized similarity.
