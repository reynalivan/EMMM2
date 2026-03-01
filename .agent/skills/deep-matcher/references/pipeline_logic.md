# Deep Matcher Pipeline Logic

## 1. Normalization (Pre-processing)

Before any match, normalize strings:

- Lowercase
- Remove special chars `[_-\.]` -> ` `
- Ascii conversion (Unidecode)

## 2. The Layers

### L1: Name Match (Fastest)

- **Input:** Folder Name.
- **Logic:** `db_name.contains(folder_name)` or vice versa.
- **Weight:** High (Matches user intent).

### L2: Token Match (Heuristic)

- **Input:** Split Folder Name into `HashSet<String>`.
- **Logic:** Intersection with DB Tags.
- **Score:** `(matches / total_tokens) * 100`.

### L3: Content Scan (IO Heavy)

- **Use:** ONLY if L1/L2 fail (< 50% confidence).
- **Logic:**
  - Walk folder (max depth 3).
  - Find `.ini` or `.ib` files.
  - Check if filename matches Character Name.

### L4: Fuzzy Match (Last Resort)

- **Use:** If specific "typo" suspected.
- **Algo:** Levenshtein Distance (crate `strsim`).
- **Threshold:** > 0.85 similarity.
