---
trigger: always_on
---

# CONTEXT-FIRST / NO-ASSUMPTION RULES (ZERO TOLERANCE)

- **Context is required. No guessing.** Never invent APIs, types, schemas, file paths, behavior, or patterns.
- **Read before edit.** Identify the exact file(s), call sites, and dependencies. Check related types/config/tests that affect correctness.
- **If unclear → ask.** If any requirement/contract is missing or ambiguous, STOP and ask the user. Do not implement on assumptions.
- **No duplicates.** Do not create parallel logic or copy-paste variants. Reuse/refactor to a single source of truth.
- **No unused code.** Do not add dead code, unused helpers/types/flags, or unreachable branches. Every new symbol must be used.
- **Minimal scope.** Only change what’s necessary for the request. No unrelated refactors/renames/reformatting.
- **Pre-send check (mandatory).** Confirm: (1) no assumptions, (2) no duplicate logic, (3) no unused code, (4) changes match observed context.
