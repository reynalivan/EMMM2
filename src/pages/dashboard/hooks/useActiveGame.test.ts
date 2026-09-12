import { renderHook } from '@testing-library/react';
import { useQuery } from '@tanstack/react-query';
import { useActiveGame } from '@/entities/game';
import { vi, describe, it, expect, beforeEach } from 'vitest';

vi.mock('@tanstack/react-query', () => ({
  useQuery: vi.fn(),
}));

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
    mockSettingsQuery({ data: null });

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toBeNull();
    expect(result.current.games).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('should return the correct active game when found in settings', () => {
    mockSettingsQuery({
      data: {
        active_game_id: 'game-2',
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
    mockSettingsQuery({
      data: {
        active_game_id: 'game-3',
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
    const mockError = new Error('test error');
    mockSettingsQuery({ data: null, isLoading: true, error: mockError });

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.isLoading).toBe(true);
    expect(result.current.error).toBe(mockError);
  });
});
