# TSDoc Standards (React)

## 1. Component Documentation
Document the component and its Props interface.

```tsx
interface ModCardProps {
  /** The mod object to display */
  mod: Mod;
  /** Callback when user toggles the enable switch */
  onToggle: (id: string, enabled: boolean) => void;
  /** Optional custom class name */
  className?: string;
}

/**
 * Displays a single mod with thumbnail and quick actions.
 *
 * @example
 * <ModCard mod={item} onToggle={handleToggle} />
 */
export const ModCard = ({ mod, onToggle }: ModCardProps) => { ... }
```

## 2. Utility Functions

```ts
/**
 * Normalizes a string for fuzzy matching.
 * Removes punctuation and converts to lowercase.
 *
 * @param input - The raw string (e.g., "Raiden Shogun [Mod]")
 * @returns Normalized string (e.g., "raidenshogunmod")
 */
export function normalizeName(input: string): string { ... }
```
