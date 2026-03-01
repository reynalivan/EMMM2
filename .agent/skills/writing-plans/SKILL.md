---
name: writing-plans
description: Use when you have a spec or requirements for a multi-step task, before touching code
---

# Writing Plans

## Overview

Write comprehensive implementation plans assuming the engineer has zero context for our codebase and questionable taste. Document everything they need to know: which files to touch for each task, code, testing, docs they might need to check, how to test it. Give them the whole plan as bite-sized tasks. DRY. YAGNI. TDD. Frequent commits.

**Announce at start:** "I'm using the writing-plans skill to create the implementation plan."

**Context:** This should be run in a dedicated task.

**Save plans to:** `.docs/plans/YYYY-MM-DD-<feature-name>.md`

## Bite-Sized Task Granularity

**Each step is one action (2-5 minutes):**

- "Write the failing test" - step
- "Run it to make sure it fails" - step
- "Implement the minimal code to make the test pass" - step
- "Run the tests and make sure they pass" - step
- "Commit" - step

## Plan Document Header

**Every plan MUST start with this header:**

```markdown
# [Feature Name] Implementation Plan

> **Goal:** [One sentence describing what this builds]

**Architecture:** [2-3 sentences about approach]

**Tech Stack:** [Key technologies/libraries]

---
```

## Task Structure

````markdown
### Task N: [Component Name]

**Files:**

- Create: `exact/path/to/file.ts`
- Modify: `exact/path/to/existing.ts:123-145`
- Test: `src/features/feature/tests/test.test.tsx`

**Step 1: Write the failing test**

```typescript
test('specific behavior', () => {
  // ...
});
```
````

**Step 2: Run test to verify it fails**

Run: `pnpm test ...`
Expected: FAIL with "function not defined"

**Step 3: Write minimal implementation**

```typescript
export function implementation() {
  // ...
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm test ...`
Expected: PASS

**Step 5: Commit**

```bash
git add .
git commit -m "feat: add specific feature"
```

## Remember

- Exact file paths always
- Complete code in plan (not "add validation")
- Exact commands with expected output
- Reference relevant skills with @ syntax
- DRY, YAGNI, TDD, frequent commits

## Execution Handoff

After saving the plan, offer execution choice:

**"Plan complete and saved to `.docs/plans/<filename>.md`. Options:**

1.  **Execute Task-by-Task** - I will follow the TDD cycle strictly.
2.  **User Review** - You review the plan first."
