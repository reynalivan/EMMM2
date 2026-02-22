---
trigger: always_on
---

# 🚫 ZERO-TRUNCATION POLICY (NON-NEGOTIABLE)

> **Purpose:** Prevent catastrophic loss of logic by never replacing code with placeholders. Better to STOP than damage.

## 1) NO PLACEHOLDERS OR SKIPS

**ABSOLUTELY FORBIDDEN** to replace or compress code with ANY placeholder:

- `// ...`, `/* ... */`, `...`
- `// previous code`, `// rest unchanged`, `// same as before`
- Any wording like "etc", "other implementation omitted".

## 2) NO ACCIDENTAL DELETION

- **Preserve 100%** of logic/structure unless explicitly instructed to delete.
- **Do NOT rewrite** from scratch unless requested.

## 3) OUTPUT REQUIRED (SAFE)

Must use ONE of:

1. **Unified diff** (`diff --git`) with full context lines.
2. **Full file output** (nothing missing).

## 4) TOKEN LIMIT PROTOCOL (STOP IMMEDIATELY)

If you suspect you cannot include the full required code:

- **STOP immediately**. Output EXACTLY: `STOP: token limit risk. Send "continue" to receive the next chunk.`
- Continue in the next message EXACTLY from where you stopped. No re-generating, no placeholders.
