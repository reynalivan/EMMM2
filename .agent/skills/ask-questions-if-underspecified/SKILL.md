---
name: ask-questions-if-underspecified
description: Guide for clarifying ambiguous user requests before starting implementation. Use when requirements, scope, or constraints are unclear.
---

# Ask Questions If Underspecified

## Goal
Avoid wasted effort by asking the **minimum set of clarifying questions** needed to define "Done".
Do not start implementing until the **Must-Have** questions are answered (or user approves assumptions).

## Workflow

### 1. Detect Ambiguity
Use this skill if ANY of the following are undefined:
-   **Objective**: What changes vs what stays same?
-   **Done Criteria**: Acceptance tests, edge cases?
-   **Scope**: Specific files, components, or users?
-   **Constraints**: Tech stack versions, performance, styling?
-   **Safety**: Data migration, rollback plan?

> If multiple valid interpretations exist, the request is **Underspecified**.

### 2. The "Must-Have" Questions
Ask 1-5 questions max. Prioritize blocking issues.
-   **Format**: Numbered list. short and scannable.
-   **Defaults**: ALWAYS suggest a "Recommended/Default" path.
-   **Multiple Choice**: Provide A/B/C options to reduce friction.

### 3. Pause & Listen
-   **Stop**: Do not write code or run destructive commands.
-   **Discovery**: You MAY run read-only discovery (e.g., `ls`, `cat`) to inform your questions.
-   **Bypass**: If the user says "Just do it", state your **Assumptions** clearly and proceed.

### 4. Confirm & Execute
Once satisfied:
1.  Restate the plan in 1-2 sentences.
2.  Start the work.

## References
-   [Question Templates](references/question_templates.md)
