---
name: code-review
description: Automated code review for pull requests using specialized review patterns. Analyzes code for quality, security, performance, and best practices. Use when reviewing code changes, PRs, or doing code audits.
---

# Code Review Skill

Standardized audit process for EMMM2.

## 1. Review Process
1.  **Security First**: Check `references/security_checklist.md`.
2.  **Performance Second**: Check `references/performance_checklist.md`.
3.  **Quality Third**: Check `references/quality_checklist.md`.

## 2. Output Format
Start with a **Summary Table**.

### 🔴 Critical (Blocker)
-   **Security**: Hardcoded secrets, SQL Injection, XSS.
-   **Stability**: Unhandled Result/Option unwraps (`.unwrap()`).
-   **Performance**: N+1 Queries in loops.

### 🟡 Warning (Request Changes)
-   **Quality**: > 350 Lines, Duplicate Logic.
-   **Performance**: React prop drilling, unnecessary re-renders.

### 🟢 Nit (Optional)
-   Naming conventions, Typos, Comment clarity.

## References
-   [Security Checklist](references/security_checklist.md)
-   [Performance Checklist](references/performance_checklist.md)
-   [Quality Checklist](references/quality_checklist.md)
