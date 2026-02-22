---
name: code-refactoring
description: Code refactoring patterns and techniques for improving code quality without changing behavior. Use for cleaning up legacy code, reducing complexity, or improving maintainability.
---

# Code Refactoring Skill

Systematic approach to cleaning code while keeping it green.

## 1. Principles
-   **Red-Green-Refactor**: Never refactor failing code. Fix it first.
-   **One Thing at a Time**: Don't mix Feature work with Refactoring.
-   **Behavior Preserving**: The external API and behavior must not change.

## 2. Project Rules (EMMM2 Context)
-   **350-Line Limit**: Split files exceeding this limit (Rule: `meta_coding_standards.md`).
-   **No Truncation**: Always rewrite the full file.
-   **Test Coverage**: Must have tests before significant refactoring.

## 3. Workflow Integration
Follow `.agent/workflows/refactor.md`:
1.  **Pre-flight**: Check tests. Save state.
2.  **Analyze**: Identify "Smells" (God Class, Deep Nesting).
3.  **Execute**: Apply specific patterns (Extract Method, Custom Hook).
4.  **Verify**: Run `cargo clippy` and `npm run lint`.

## References
-   [Rust Refactoring Patterns](references/rust_refactor.md)
-   [React Refactoring Patterns](references/react_refactor.md)
-   [Common Code Smells](references/common_smells.md)
