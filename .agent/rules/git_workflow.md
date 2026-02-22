---
trigger: model_decision
description: When the user asks to commit code, create branches, merge PRs, or manage git history.
---

# 🐙 Git Workflow

> **Goal:** Clean, linear, atomic history.

## 1. Branching

- `main`: Prod-ready.
- `feat/name`: New features.
- `fix/issue`: Bug fixes.

## 2. Conventional Commits

- `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`.
- **Rule:** Imperative ("Add" not "Added"). No trailing dot.

## 3. Atomic Commits

- **Rule:** One logical change per commit.
- **Test:** If "and" is needed in message, split it.

## 4. PR Process

- **Title:** Matches Commit.
- **Merge:** Squash & Merge (Preferred).
