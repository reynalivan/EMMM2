---
trigger: model_decision
description: Security & Git Standards - Secrets handling, scanning, and commit workflows.
---

- Secrets: API keys/credentials/DB passwords in .env ONLY.
- Protection: PINs MUST use Argon2 hashing. API keys in OS Keyring.
- OS: Rust (dotenv/std::env); TS (import.meta.env.VITE\_\*).
- Detection: Check console.log/comments/consts before commit.
- Scanning: Run mcp_narsil-mcp_check_owasp_top10 on I/O/Auth changes.
- History: Linear, atomic. 1 logical change = 1 commit.
- Conv Commits: feat:/fix:/docs:/chore:/refactor:. Imperative mood.
- PR: Squash & Merge. Title matches commit.
- Branches: feat/ (Feature), fix/ (Bug), main (Production).
