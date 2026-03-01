---
name: changelog-generator
description: Automatically creates user-facing changelogs from git history. Use for Release Notes, Weekly Updates, or App Store descriptions.
---

# Changelog Generator Skill

Transform raw git commits into polished, customer-facing release notes.

## Goal

Turn "Technical Noise" (commits) into "User Value" (Changelog).

## Workflow

### 1. Analysis (The "Git Scan")

Run `git log --since="<date>"` or `git log <tag>..HEAD`.

- **Ignore**: Merge commits, chore, internal refactors, tests.
- **Focus**: `feat`, `fix`, `perf`, `ui`.

### 2. Categorization

Group changes into these buckets:

1.  **✨ New Features**: Major additions users can see/use.
2.  **🔧 Improvements**: Performance, UI tweaks, QoL.
3.  **🐛 Bug Fixes**: Validated repairs.
4.  **⚠️ Breaking Changes**: anything requiring user action.

### 3. Translation (Dev -> User)

Rewrite technical jargon into benefits.

- ❌ `refactor(auth): switch to argon2id`
- ✅ **Enhanced Security**: Upgraded password hashing for better protection.

## References

- [Changelog Style Guide](references/style_guide.md)
