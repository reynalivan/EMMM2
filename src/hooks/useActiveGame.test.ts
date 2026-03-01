import { renderHook } from '@testing-library/react';
import { useActiveGame } from './useActiveGame';
import { useSettings } from './useSettings';
import { useAppStore } from '../stores/useAppStore';
import { vi, describe, it, expect, beforeEach } from 'vitest';

vi.mock('./useSettings', () => ({
  useSettings: vi.fn(),
}));

vi.mock('../stores/useAppStore', () => ({
  useAppStore: vi.fn(),
}));

describe('useActiveGame', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should return null activeGame when there is no activeGameId', () => {
    vi.mocked(useAppStore).mockReturnValue({ activeGameId: null } as unknown as ReturnType<
      typeof useAppStore
    >);
    vi.mocked(useSettings).mockReturnValue({
      settings: null,
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useSettings>);

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toBeNull();
    expect(result.current.games).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('should return the correct active game when found in settings', () => {
    vi.mocked(useAppStore).mockReturnValue({ activeGameId: 'game-2' } as unknown as ReturnType<
      typeof useAppStore
    >);
    vi.mocked(useSettings).mockReturnValue({
      settings: {
        games: [
          { id: 'game-1', name: 'Game 1' },
          { id: 'game-2', name: 'Game 2' },
        ],
      },
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useSettings>);

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toEqual({ id: 'game-2', name: 'Game 2' });
    expect(result.current.games).toHaveLength(2);
  });

  it('should return null if activeGameId is set but game not found in settings', () => {
    vi.mocked(useAppStore).mockReturnValue({ activeGameId: 'game-3' } as unknown as ReturnType<
      typeof useAppStore
    >);
    vi.mocked(useSettings).mockReturnValue({
      settings: {
        games: [
          { id: 'game-1', name: 'Game 1' },
          { id: 'game-2', name: 'Game 2' },
        ],
      },
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useSettings>);

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.activeGame).toBeNull();
  });

  it('should pass through isLoading and error from useSettings', () => {
    vi.mocked(useAppStore).mockReturnValue({ activeGameId: null } as unknown as ReturnType<
      typeof useAppStore
    >);
    const mockError = new Error('test error');

    vi.mocked(useSettings).mockReturnValue({
      settings: null,
      isLoading: true,
      error: mockError,
    } as unknown as ReturnType<typeof useSettings>);

    const { result } = renderHook(() => useActiveGame());

    expect(result.current.isLoading).toBe(true);
    expect(result.current.error).toBe(mockError);
  });
});
