---
trigger: always_on
description: Zero-Truncation Policy - Strict enforcement of code completeness and prevention of logical loss via placeholders.
---

- No Placeholders: FORBIDDEN to replace/compress code with //..., /*...*/, etc.
- No Accidental Deletion: Preserve 100% logic unless deleting explicitly. No rewrites.
- Output: Use Unified diff (diff --git) or Full file (nothing missing).
- Token Protocol: If risk, STOP immediately. Output: `STOP: token limit risk. Send "continue" to receive the next chunk.`
- Continuation: Resume EXACTLY where stopped. No re-generating/skipping.
