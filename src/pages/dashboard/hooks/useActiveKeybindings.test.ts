import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useActiveKeybindings } from './useActiveKeybindings';
import { useActiveGame } from '@/entities/game';
import { invoke } from '@tauri-apps/api/core';
import { createWrapper } from '../../../tests/testing/test-utils';
import { useAppStore } from '@/app/store';
import type { GameActivationPhase } from '@/shared/api/tauri/bindings';

// Restore real @tanstack/react-query — the global setupTests stub
// replaces useQuery with a no-op, which prevents queryFn from running.
vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: vi.fn(),
}));

describe('useActiveKeybindings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      activeGameId: 'game-1',
      requestedGameId: null,
      gameActivationByGame: {},
      diskReconcileByGame: {},
    });
  });

  it('should not fetch if activeGame is null', () => {
    useAppStore.setState({ activeGameId: null });
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: null } as unknown as ReturnType<
      typeof useActiveGame
    >);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });

    expect(result.current.isLoading).toBe(false); // Query stays disabled
    expect(invoke).not.toHaveBeenCalled();
  });

  it('should fetch and return keybindings when activeGame is present', async () => {
    setActivation('ready');
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-1' },
    } as unknown as ReturnType<typeof useActiveGame>);
    const mockBindings = [{ id: '1', key: 'F1', action: 'test' }];
    vi.mocked(invoke).mockResolvedValue(mockBindings);

    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invoke).toHaveBeenCalledWith('get_active_keybindings', { gameId: 'game-1' });
    expect(result.current.keybindings).toEqual(mockBindings);
    expect(result.current.isError).toBe(false);
  });

  it('should handle errors during fetch', async () => {
    useAppStore.setState({ activeGameId: 'game-2' });
    setActivation('ready', 1, 'game-2');
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-2' },
    } as unknown as ReturnType<typeof useActiveGame>);
    vi.mocked(invoke).mockRejectedValue(new Error('Fetch failed'));

    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.keybindings).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('defers scanning until the existing activation is ready', async () => {
    setActivation('syncing');
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
      typeof useActiveGame
    >);
    vi.mocked(invoke).mockResolvedValue([]);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    expect(result.current.isLoading).toBe(true);
    expect(invoke).not.toHaveBeenCalled();
    act(() => setActivation('ready'));
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('waits when activation has not arrived yet', () => {
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
      typeof useActiveGame
    >);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    expect(result.current.isLoading).toBe(true);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('settles a removed selected game without leaving an eternal skeleton', () => {
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: null,
      isLoading: false,
      error: null,
      games: [],
    });
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isError).toBe(true);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('settles a later source-unavailable result and hides previously cached bindings', async () => {
    setActivation('ready');
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
      typeof useActiveGame
    >);
    vi.mocked(invoke).mockResolvedValue([{ key: 'F1' }]);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    await waitFor(() => expect(result.current.keybindings).toEqual([{ key: 'F1' }]));
    act(() => useAppStore.getState().setDiskSourceUnavailable('game-1', 'Drive disconnected'));
    expect(result.current.keybindings).toEqual([]);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isError).toBe(true);
  });

  it('does not publish a previous game response over the newly selected game', async () => {
    let completeOld!: (value: unknown) => void;
    setActivation('ready');
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
      typeof useActiveGame
    >);
    vi.mocked(invoke)
      .mockReturnValueOnce(
        new Promise((resolve) => {
          completeOld = resolve;
        }),
      )
      .mockResolvedValue([{ key: 'F2' }]);
    const { result } = renderHook(() => useActiveKeybindings(), {
      wrapper: createWrapper,
    });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    act(() => {
      useAppStore.setState({ activeGameId: 'game-2' });
      vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-2' } } as ReturnType<
        typeof useActiveGame
      >);
      setActivation('ready', 2, 'game-2');
    });
    await waitFor(() => expect(result.current.keybindings).toEqual([{ key: 'F2' }]));
    await act(async () => completeOld([{ key: 'F1' }]));
    expect(result.current.keybindings).toEqual([{ key: 'F2' }]);
  });

  it.each(['failed', 'source_unavailable'] as const)('settles %s without scanning', (phase) => {
    setActivation(phase);
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
      typeof useActiveGame
    >);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isError).toBe(true);
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each(['game-1', 'game-2'])(
    'hides previous data during a pending %s activation',
    async (gameId) => {
      setActivation('ready');
      vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
        typeof useActiveGame
      >);
      vi.mocked(invoke).mockResolvedValue([{ key: 'F1' }]);
      const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
      await waitFor(() => expect(result.current.keybindings).toEqual([{ key: 'F1' }]));
      act(() => useAppStore.setState({ requestedGameId: gameId }));
      expect(result.current.keybindings).toEqual([]);
      expect(result.current.isLoading).toBe(true);
      expect(invoke).toHaveBeenCalledTimes(1);
    },
  );

  it('refreshes after new same-game recovery and ignores a late previous scan', async () => {
    let completeOld!: (value: unknown) => void;
    setActivation('ready');
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: { id: 'game-1' } } as ReturnType<
      typeof useActiveGame
    >);
    vi.mocked(invoke)
      .mockReturnValueOnce(
        new Promise((resolve) => {
          completeOld = resolve;
        }),
      )
      .mockResolvedValue([{ key: 'F2' }]);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    act(() => setActivation('syncing', 2));
    expect(result.current.isLoading).toBe(true);
    await act(async () => completeOld([{ key: 'F1' }]));
    expect(result.current.keybindings).toEqual([]);
    act(() => setActivation('ready', 2));
    await waitFor(() => expect(result.current.keybindings).toEqual([{ key: 'F2' }]));
    expect(invoke).toHaveBeenCalledTimes(2);
  });
});

function setActivation(phase: GameActivationPhase, generation = 1, gameId = 'game-1') {
  useAppStore.setState({
    gameActivationByGame: {
      ...useAppStore.getState().gameActivationByGame,
      [gameId]: {
        game_id: gameId,
        generation,
        phase,
        reconcile_revision: null,
        runtime_sync_generation: null,
        error: null,
      },
    },
  });
}
