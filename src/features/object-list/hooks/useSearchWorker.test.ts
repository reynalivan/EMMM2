import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useSearchWorker } from './useSearchWorker';

describe('useSearchWorker', () => {
  beforeEach(() => {
    vi.stubGlobal('Worker', undefined);
  });

  it('keeps the same filteredIds reference when repeated search results are identical', () => {
    const { result } = renderHook(() => useSearchWorker());
    const items = [
      { id: '1', name: 'Diluc Skin' },
      { id: '2', name: 'Amber Skin' },
    ];

    act(() => {
      result.current.search(items, 'skin');
    });

    const firstResult = result.current.filteredIds;
    expect(firstResult).not.toBeNull();

    act(() => {
      result.current.search(items, 'skin');
    });

    expect(result.current.filteredIds).toBe(firstResult);
  });

  it('keeps null result stable for repeated empty queries', () => {
    const { result } = renderHook(() => useSearchWorker());

    act(() => {
      result.current.search([], '');
    });

    expect(result.current.filteredIds).toBeNull();

    act(() => {
      result.current.search([], '   ');
    });

    expect(result.current.filteredIds).toBeNull();
  });
});
