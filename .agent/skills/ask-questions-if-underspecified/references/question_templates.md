# Question Templates

Use these patterns to get fast, clear answers.

## 1. Multiple Choice (Low Friction)
"I need to know how to handle X. Which approach do you prefer?"

> 1) **Option A (Recommended)**: Description of A.
> 2) **Option B**: Description of B.
> 3) **Option C**: Custom (please specify).

*Reply with `1` to select the recommended option.*

## 2. The "Defaults" Offer
"To proceed, I need to clarify a few things. Here are my recommended defaults:"

> -   **Scope**: Only file X and Y.
> -   **Style**: Match existing project eslint rules.
> -   **Error Handling**: Log to console and ignore.

*Reply `defaults` to accept these, or specify changes.*

## 3. Scope Clarification
"This change could be minimal or extensive:"

> 1) **Surgical**: Only fix the specific bug in `utils.ts`.
> 2) **Refactor**: Rewrite `utils.ts` and update all 5 call sites.

*Which scope should I target?*

## 4. Constraint Check
"Are there constraints I should know?"
-   **Versions**: Support Node 14? Or just 18+?
-   **Libs**: Can I add `zod`? Or use vanilla JS?
-   **Performance**: Is this hot path (needs optimization) or background task?

## 5. Technical Decision
"I found two ways to implement this:"
> 1) **Store logic**: Put data in `Zustand` (Global).
> 2) **Component logic**: Keep state local to `Modal.tsx`.

*Option 1 is better for sharing, Option 2 is simpler. Thoughts?*
