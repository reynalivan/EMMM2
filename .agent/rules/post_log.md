---
trigger: always_on
description: Post-Implementation Logging
---

## Post-Implementation Logging (MANDATORY)

After EVERY completed implementation session:

### Create File

- Location: `.docs/history/`
- Filename:
  - `yyyymmddxxxx-[short-title-summary].md`
  - `xxxx` = incremental/unique
  - lowercase, hyphen-separated

## Content (STRICT, concise)

### Title

- Short summary of change

### Context

- Why this change was needed

### Changes

- Key logic/behavior changes (before → after)
- No diff dump

### Impacted Files

- List ALL affected files
- Group by type if possible

Example:

- `src/auth/login.ts` (modified)
- `src/auth/useAuth.ts` (modified)
- `src/types/auth.ts` (added)
- `src/utils/token.ts` (removed)

### Goal

- What the system achieves now

### Impact

- Side effects / affected flows
- Performance implications (if any)
- Breaking changes (if any)

### Notes (optional)

- Key decisions / trade-offs

## Rules

- No verbose explanation
- No full code
- No fake/assumed changes
- Skip trivial formatting-only edits
- Must be readable <1 minute

## Focus

- WHAT changed
- WHY it changed
- WHICH FILES are impacted
- RESULT after change
