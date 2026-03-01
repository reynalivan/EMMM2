---
name: virt-grid
description: High-performance React virtualization for large lists and grids (>50 items). Essential for FolderGrid, ModList, and heavy data tables.
---

# Virtual Grid Skill

Optimize rendering performance using `@tanstack/react-virtual`.

## When to use

- **Explorer Views**: `FolderGrid` (Epic 4).
- **ObjectLists**: `ModList` (Epic 3).
- **Log Streams**: `ConsoleView` (Epic 13).
- **Any list > 50 items**.

## Core Rules

### 1. The Container Rule

- **Height**: Container **MUST** have an explicit height (e.g., `h-full`, `500px`).
- **Overflow**: CSS must have `overflow-y: auto`.
- **Relative**: Parent must be `relative` for absolute positioning of items.

### 2. React 19 Compatibility

- **FlushSync**: Set `useFlushSync: false` in options to avoid warnings in React 19.
- **Keys**: Use stable, unique IDs (DB IDs), never array index.

### 3. Infinite Scroll Pattern

- **Integration**: Combine with `TanStack Query` (`useInfiniteQuery`).
- **Trigger**: Use standard `virtualizer.getVirtualItems()` to detect end of list.

## References

- [Best Practices & React 19](references/best_practices.md)

## Examples

- [Infinite List (Chat/Logs)](examples/virtual_list.tsx)
- [Responsive Grid (Explorer)](examples/virtual_grid.tsx)
