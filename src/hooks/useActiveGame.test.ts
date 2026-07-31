import { renderHook } from '@testing-library/react';
import { useQuery } from '@tanstack/react-query';
import { useActiveGame } from './useActiveGame';
import { useAppStore } from '../stores/useAppStore';
import { vi, describe, it, expect, beforeEach } from 'vitest';

vi.mock('@tanstack/react-query', () => ({
  useQuery: vi.fn(),
}));

vi.mock('../stores/useAppStore', () => ({
  useAppStore: vi.fn(),
}));

function mockActiveGameId(activeGameId: string | null) {
  vi.mocked(useAppStore).mockImplementation((selector: unknown) =>
    (selector as (state: { activeGameId: string | null }) => unknown)({ activeGameId }),
  );
}

function mockSettingsQuery(result: { data?: unknown; isLoading?: boolean; error?: unknown }) {
  vi.mocked(useQuery).mockReturnValue({
    data: result.data ?? null,
    isLoading: result.isLoading ?? false,
    error: result.error ?? null,
  } as unknown as ReturnType<typeof useQuery>);
}

describe('useActiveGame', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should return null activeGame when there is no activeGameId', () => {
    mockActiveGameId(null);
    mockSettingsQuery({ data: null });

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toBeNull();
    expect(result.current.games).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('should return the correct active game when found in settings', () => {
    mockActiveGameId('game-2');
    mockSettingsQuery({
      data: {
        games: [
          { id: 'game-1', name: 'Game 1' },
          { id: 'game-2', name: 'Game 2' },
        ],
      },
    });

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toEqual({ id: 'game-2', name: 'Game 2' });
    expect(result.current.games).toHaveLength(2);
  });

  it('should return null if activeGameId is set but game not found in settings', () => {
    mockActiveGameId('game-3');
    mockSettingsQuery({
      data: {
        games: [
          { id: 'game-1', name: 'Game 1' },
          { id: 'game-2', name: 'Game 2' },
        ],
      },
    });

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toBeNull();
  });

  it('should pass through isLoading and error from the settings query', () => {
    mockActiveGameId(null);
    const mockError = new Error('test error');
    mockSettingsQuery({ data: null, isLoading: true, error: mockError });

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.isLoading).toBe(true);
    expect(result.current.error).toBe(mockError);
  });
});
