import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useFolderNavigation } from './useFolderNavigation';

describe('useFolderNavigation', () => {
  const mockItems = Array.from({ length: 10 }, (_, i) => ({
    path: `/path/to/mod-${i}`,
    name: `mod-${i}`,
  }));

  const defaultProps = {
    items: mockItems,
    gridColumns: 4,
    onNavigate: vi.fn(),
    onSelectionChange: vi.fn(),
    onDelete: vi.fn(),
    onRename: vi.fn(),
    onGoUp: vi.fn(),
    getId: (item: { path: string }) => item.path,
  };

  it('initializes with no focus', () => {
    const { result } = renderHook(() => useFolderNavigation(defaultProps));
    expect(result.current.focusedId).toBeNull();
  });

  it('ArrowRight moves focus next', () => {
    const { result } = renderHook(() => useFolderNavigation(defaultProps));

    // Initial focus set manually or by first keypress?
    // Usually first keypress selects first item if nothing selected.
    act(() => {
      const event = new KeyboardEvent('keydown', { key: 'ArrowRight' });
      result.current.handleKeyDown(event);
    });

    expect(result.current.focusedId).toBe(mockItems[0].path);

    act(() => {
      const event = new KeyboardEvent('keydown', { key: 'ArrowRight' });
      result.current.handleKeyDown(event);
    });

    expect(result.current.focusedId).toBe(mockItems[1].path);
  });

  it('ArrowDown moves down by column count (Grid Mode)', () => {
    const { result } = renderHook(() => useFolderNavigation(defaultProps));

    // Focus 0
    act(() => {
      result.current.setFocusedId(mockItems[0].path);
    });

    act(() => {
      const event = new KeyboardEvent('keydown', { key: 'ArrowDown' });
      result.current.handleKeyDown(event);
    });

    // 0 + 4 = 4
    expect(result.current.focusedId).toBe(mockItems[4].path);
  });

  it('Enter navigates to focused item', () => {
    const { result } = renderHook(() => useFolderNavigation(defaultProps));

    act(() => {
      result.current.setFocusedId(mockItems[2].path);
    });

    act(() => {
      const event = new KeyboardEvent('keydown', { key: 'Enter' });
      result.current.handleKeyDown(event);
    });

    expect(defaultProps.onNavigate).toHaveBeenCalledWith(mockItems[2]);
  });

  it('Backspace calls onGoUp', () => {
    const { result } = renderHook(() => useFolderNavigation(defaultProps));

    act(() => {
      const event = new KeyboardEvent('keydown', { key: 'Backspace' });
      result.current.handleKeyDown(event);
    });

    expect(defaultProps.onGoUp).toHaveBeenCalled();
  });

  it('F2 calls onRename', () => {
    const { result } = renderHook(() => useFolderNavigation(defaultProps));

    act(() => {
      result.current.setFocusedId(mockItems[3].path);
    });

    act(() => {
      const event = new KeyboardEvent('keydown', { key: 'F2' });
      result.current.handleKeyDown(event);
    });

    expect(defaultProps.onRename).toHaveBeenCalledWith(mockItems[3]);
  });
});
