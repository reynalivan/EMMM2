---
name: deep-matcher
description: Logic for identifying Mods using a 4-Layer Pipeline (Name -> Token -> Content -> Fuzzy). Use when: (1) Scanning a new directory like `/Mods`, (2) Re-scanning a specific folder for metadata updates, (3) Identifying if a folder is a valid "3DMigoto Mod" based on content, or (4) Debugging why a mod isn't matching correctly.
---

# Deep Matcher Skill

Implements the "Brain" of the Mod Manager. Matches filesystem folders to known Mod Metadata using a weighted 4-layer pipeline.

## Core Pipeline

Follow this 4-Layer Pipeline strictly. Do not skip steps.

1.  **L1: Name Match** (Exact/Contains) - Fast & High Confidence.
2.  **L2: Token Match** (Tags) - Heuristic based on directory words.
3.  **L3: Content Scan** (File sniffing) - IO Heavy, checks for `.ini`/`.ib` files.
4.  **L4: Fuzzy Match** (Levenshtein) - Fallback for typos.

> **Detailed Logic:** See [pipeline.md](references/pipeline.md) for weightings and normalization rules.

## Performance Constraints

-   **Threading:** MUST run inside `tokio::spawn_blocking` or `rayon` thread pool. Never block the main thread.
-   **Recursion:** Limit `WalkDir` to max depth **3** to prevent deep scanning hang.
-   **Caching:** Hash strings (Name/Tokens) once during initialization, reuse for comparisons.

## Implementation Pattern

Use the standard builder/scoring pattern for Rust.

> **Code Example:** See [usage_pattern.rs](examples/usage_pattern.rs)
