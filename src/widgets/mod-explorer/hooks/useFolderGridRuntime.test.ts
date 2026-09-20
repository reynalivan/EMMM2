import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  EXPLORER_SEARCH_DEBOUNCE_MS,
  getPreviousExplorerSubPath,
  useDebouncedExplorerSearchQuery,
} from './useFolderGridRuntime';

afterEach(() => {
  vi.useRealTimers();
});

describe('getPreviousExplorerSubPath', () => {
  it('targets the direct parent view for a breadcrumb folder switcher', () => {
    expect(getPreviousExplorerSubPath('SkinSelectImpact/Aglaea')).toBe('SkinSelectImpact');
    expect(getPreviousExplorerSubPath('E:/Mods/SkinSelectImpact/Aglaea')).toBe(
      'E:/Mods/SkinSelectImpact',
    );
    expect(getPreviousExplorerSubPath('SkinSelectImpact')).toBeUndefined();
  });

  it('debounces only the backend explorer search value', () => {
    vi.useFakeTimers();
    const { result, rerender } = renderHook(
      ({ search }) => useDebouncedExplorerSearchQuery(search),
      { initialProps: { search: '' } },
    );

    rerender({ search: 'amber' });
    expect(result.current).toBe('');

    act(() => vi.advanceTimersByTime(EXPLORER_SEARCH_DEBOUNCE_MS - 1));
    expect(result.current).toBe('');

    act(() => vi.advanceTimersByTime(1));
    expect(result.current).toBe('amber');
  });
});
