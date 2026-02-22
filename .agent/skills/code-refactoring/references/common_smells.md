# Common Code Smells & Fixes

## 1. Long Method / File (> 350 Lines)
-   **Fix**: Extract Method / Extract Component.
-   **Fix**: Move logic to Service (Backend) or Hook (Frontend).

## 2. Deep Nesting (Arrow Code)
-   **Fix**: Invert `if` to Guard Clauses.
-   **Fix**: Extract conditional blocks to functions.

## 3. Primitive Obsession
-   **Fix**: Replace `String` (e.g., email) with a strict Type/Struct `Email(String)`.
-   **Fix**: Use Enums instead of magic strings/numbers.

## 4. Feature Envy
-   **Fix**: Move the method to the class/struct where the data lives.
