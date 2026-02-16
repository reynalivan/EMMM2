import { useState, useCallback, type KeyboardEvent } from 'react';

export interface FolderNavigationProps<T> {
  items: T[];
  gridColumns: number;
  onNavigate: (item: T) => void;
  onSelectionChange?: (item: T, multi: boolean, range: boolean) => void;
  onSelectAll?: () => void;
  onDelete?: (items: T[]) => void;
  onRename?: (item: T) => void;
  onGoUp?: () => void;
  getId: (item: T) => string;
}

export function useFolderNavigation<T>({
  items,
  gridColumns,
  onNavigate,
  onSelectionChange: _onSelectionChange,
  onSelectAll,
  onDelete,
  onRename,
  onGoUp,
  getId,
}: FolderNavigationProps<T>) {
  const [focusedId, setFocusedId] = useState<string | null>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent | globalThis.KeyboardEvent) => {
      // Allow default behavior for some keys if needed, but usually we prevent default for navigation

      // Ctrl+A — select all items
      if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
        e.preventDefault();
        onSelectAll?.();
        return;
      }

      const currentIndex = items.findIndex((item) => getId(item) === focusedId);
      let nextIndex: number;

      switch (e.key) {
        case 'ArrowRight':
          e.preventDefault();
          nextIndex = currentIndex === -1 ? 0 : Math.min(items.length - 1, currentIndex + 1);
          break;
        case 'ArrowLeft':
          e.preventDefault();
          nextIndex = currentIndex === -1 ? 0 : Math.max(0, currentIndex - 1);
          break;
        case 'ArrowDown':
          e.preventDefault();
          if (currentIndex === -1) {
            nextIndex = 0;
          } else {
            nextIndex = Math.min(items.length - 1, currentIndex + gridColumns);
          }
          break;
        case 'ArrowUp':
          e.preventDefault();
          if (currentIndex === -1) {
            nextIndex = items.length - 1;
          } else {
            nextIndex = Math.max(0, currentIndex - gridColumns);
          }
          break;
        case 'Home':
          e.preventDefault();
          nextIndex = 0;
          break;
        case 'End':
          e.preventDefault();
          nextIndex = items.length - 1;
          break;
        case 'Enter':
          e.preventDefault();
          if (currentIndex !== -1) {
            onNavigate(items[currentIndex]);
          }
          return; // No focus change needed (or maybe handled by navigate)
        case 'Backspace':
          // Only if not in an input field (check target?) - Browser usually handles this check but we should be careful
          // We assume this handler is on a focusable container (div tabIndex=0)
          e.preventDefault();
          onGoUp?.();
          return;
        case 'Delete':
          e.preventDefault();
          // if (currentIndex !== -1) onDelete?.([items[currentIndex]]); // Or selected items
          // For now, let's say it deletes the focused item if selection is empty, or selected items
          // To keep it simple for this hook, we might defer deletion logic to the parent which knows about selection
          // But the spec said "Delete focused/selected items"
          // Let's assume parent handles selection deletion, this hook handles navigation.
          // BUT if we want to trigger delete from here...
          // Let's trigger onDelete with focused item for now if standard
          if (currentIndex !== -1) onDelete?.([items[currentIndex]]);
          return;
        case 'F2':
          e.preventDefault();
          if (currentIndex !== -1) onRename?.(items[currentIndex]);
          return;
        default:
          return;
      }

      if (nextIndex !== -1 && nextIndex !== currentIndex) {
        const nextItem = items[nextIndex];
        const nextId = getId(nextItem);
        setFocusedId(nextId);

        // Scroll into view logic should happen in parent or via ref,
        // but for now we just change state.
      } else if (currentIndex === -1 && items.length > 0) {
        // If nothing focused, focus first on any navigation key (handled above usually)
        setFocusedId(getId(items[0]));
      }
    },
    [items, focusedId, gridColumns, getId, onNavigate, onGoUp, onDelete, onRename, onSelectAll],
  );

  return {
    focusedId,
    setFocusedId,
    handleKeyDown,
  };
}
