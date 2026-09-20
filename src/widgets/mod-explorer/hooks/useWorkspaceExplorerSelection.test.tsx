import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { WorkspaceExplorerQuery } from '@/entities/workspace';
import { useWorkspaceExplorerSelection } from './useWorkspaceExplorerSelection';

function query(searchQuery: string | null): WorkspaceExplorerQuery {
  return {
    game_id: 'game-1',
    explorer_sub_path: 'Characters',
    search_query: searchQuery,
    sort_field: 'name',
    sort_order: 'asc',
    safety_filter: 'all',
  };
}

describe('useWorkspaceExplorerSelection', () => {
  it('stores Select All as all-matching with exclusions', () => {
    const setExplicitPaths = vi.fn();
    const { result } = renderHook(() =>
      useWorkspaceExplorerSelection({
        query: query(null),
        listingRevision: 'revision-1',
        totalMatching: 100_000,
        loadedPaths: [],
        explicitPaths: new Set(),
        setExplicitPaths,
        clearExplicitPaths: vi.fn(),
      }),
    );

    act(() => result.current.selectAllMatching());
    act(() => result.current.togglePath('C:/Mods/skip', true));

    expect(result.current.selection).toMatchObject({
      mode: 'all_matching',
      listingRevision: 'revision-1',
    });
    expect(result.current.selectedCount).toBe(99_999);
    expect(result.current.isPathSelected('C:/Mods/skip')).toBe(false);
    expect(setExplicitPaths).toHaveBeenCalledWith(new Set());
  });

  it('resets selection when the query scope changes', async () => {
    const clearExplicitPaths = vi.fn();
    const { result, rerender } = renderHook(
      ({ workspaceQuery }) =>
        useWorkspaceExplorerSelection({
          query: workspaceQuery,
          listingRevision: 'revision-1',
          totalMatching: 3,
          loadedPaths: [],
          explicitPaths: new Set(),
          setExplicitPaths: vi.fn(),
          clearExplicitPaths,
        }),
      { initialProps: { workspaceQuery: query(null) } },
    );
    act(() => result.current.selectAllMatching());

    rerender({ workspaceQuery: query('amber') });

    await waitFor(() => {
      expect(result.current.selection.mode).toBe('explicit');
      expect(result.current.selectedCount).toBe(0);
      expect(clearExplicitPaths).toHaveBeenCalledOnce();
    });
  });

  it('clears selection as soon as the raw search scope changes', async () => {
    const clearExplicitPaths = vi.fn();
    const { result, rerender } = renderHook(
      ({ rawSearch }) =>
        useWorkspaceExplorerSelection({
          query: query(null),
          selectionScopeKey: rawSearch,
          listingRevision: 'revision-1',
          totalMatching: 3,
          loadedPaths: [],
          explicitPaths: new Set(),
          setExplicitPaths: vi.fn(),
          clearExplicitPaths,
        }),
      { initialProps: { rawSearch: '' } },
    );
    act(() => result.current.selectAllMatching());

    rerender({ rawSearch: 'amber' });

    await waitFor(() => {
      expect(result.current.selection.mode).toBe('explicit');
      expect(result.current.selectedCount).toBe(0);
      expect(clearExplicitPaths).toHaveBeenCalledOnce();
    });
  });

  it('clears all-matching selection when the listing revision changes', async () => {
    const clearExplicitPaths = vi.fn();
    const { result, rerender } = renderHook(
      ({ listingRevision }) =>
        useWorkspaceExplorerSelection({
          query: query(null),
          listingRevision,
          totalMatching: 3,
          loadedPaths: [],
          explicitPaths: new Set(),
          setExplicitPaths: vi.fn(),
          clearExplicitPaths,
        }),
      { initialProps: { listingRevision: 'revision-1' } },
    );
    act(() => result.current.selectAllMatching());

    rerender({ listingRevision: 'revision-2' });

    await waitFor(() => {
      expect(result.current.selection.mode).toBe('explicit');
      expect(result.current.selectedCount).toBe(0);
      expect(clearExplicitPaths).toHaveBeenCalledOnce();
    });
  });

  it('tracks refreshed totals without expanding all-matching selection', async () => {
    const { result, rerender } = renderHook(
      ({ totalMatching }) =>
        useWorkspaceExplorerSelection({
          query: query(null),
          listingRevision: 'revision-1',
          totalMatching,
          loadedPaths: [],
          explicitPaths: new Set(),
          setExplicitPaths: vi.fn(),
          clearExplicitPaths: vi.fn(),
        }),
      { initialProps: { totalMatching: 100_000 } },
    );
    act(() => result.current.selectAllMatching());
    act(() => result.current.togglePath('C:/Mods/skip', true));

    rerender({ totalMatching: 100_010 });

    await waitFor(() => expect(result.current.selectedCount).toBe(100_009));
    expect(result.current.selection).toMatchObject({ mode: 'all_matching' });
  });

  it('drops exclusions that no longer exist once the full result set is loaded', async () => {
    const { result, rerender } = renderHook(
      ({ loadedPaths }) =>
        useWorkspaceExplorerSelection({
          query: query(null),
          listingRevision: 'revision-1',
          totalMatching: 1,
          loadedPaths,
          explicitPaths: new Set(),
          setExplicitPaths: vi.fn(),
          clearExplicitPaths: vi.fn(),
        }),
      { initialProps: { loadedPaths: ['C:/Mods/A'] } },
    );
    act(() => result.current.selectAllMatching());
    act(() => result.current.togglePath('C:/Mods/A', true));
    expect(result.current.selectedCount).toBe(0);

    rerender({ loadedPaths: ['C:/Mods/B'] });

    await waitFor(() => expect(result.current.selectedCount).toBe(1));
    expect(result.current.isPathSelected('C:/Mods/B')).toBe(true);
  });

  it('removes only completed paths from an explicit selection', () => {
    const setExplicitPaths = vi.fn();
    const { result } = renderHook(() =>
      useWorkspaceExplorerSelection({
        query: query(null),
        listingRevision: 'revision-1',
        totalMatching: 3,
        loadedPaths: ['C:/Mods/A', 'C:/Mods/B', 'C:/Mods/C'],
        explicitPaths: new Set(['C:/Mods/A', 'C:/Mods/B', 'C:/Mods/C']),
        setExplicitPaths,
        clearExplicitPaths: vi.fn(),
      }),
    );

    act(() => result.current.removePaths(['C:/Mods/A', 'C:/Mods/C']));

    expect(setExplicitPaths).toHaveBeenCalledWith(new Set(['C:/Mods/B']));
  });
});
