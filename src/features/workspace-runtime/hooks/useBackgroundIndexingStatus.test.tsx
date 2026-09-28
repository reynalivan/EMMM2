import { act, renderHook, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  OnboardingIndexingBackgroundStatus,
  OnboardingIndexingSnapshotProgress,
} from '@/shared/api/tauri/bindings';
import { useBackgroundIndexingStatus } from './useBackgroundIndexingStatus';

const getStatus = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: { getOnboardingIndexingBackgroundStatus: getStatus },
}));

const handlers = new Map<string, (event: { payload: unknown }) => void>();

function emit<T>(event: string, payload: T): void {
  const handler = handlers.get(event);
  if (!handler) throw new Error(`Missing listener: ${event}`);
  act(() => handler({ payload }));
}

describe('useBackgroundIndexingStatus', () => {
  beforeEach(() => {
    handlers.clear();
    getStatus.mockReset();
    vi.mocked(listen).mockImplementation(async (event, handler) => {
      handlers.set(event, handler as (event: { payload: unknown }) => void);
      return () => handlers.delete(event);
    });
  });

  it('keeps a newer event when an older status snapshot returns afterward', async () => {
    let resolveSnapshot!: (value: OnboardingIndexingBackgroundStatus[]) => void;
    getStatus.mockReturnValue(
      new Promise<OnboardingIndexingBackgroundStatus[]>((resolve) => {
        resolveSnapshot = resolve;
      }),
    );
    const { result } = renderHook(() => useBackgroundIndexingStatus());
    await waitFor(() => expect(getStatus).toHaveBeenCalledOnce());

    emit<OnboardingIndexingBackgroundStatus>('onboarding_indexing:background_status', {
      session_id: 'session-1',
      completed_games: 1,
      total_games: 1,
      games: [{ game_id: 'game-1', phase: 'Ready' }],
    });
    resolveSnapshot([
      {
        session_id: 'session-1',
        completed_games: 0,
        total_games: 1,
        games: [{ game_id: 'game-1', phase: 'Preparing' }],
      },
    ]);

    await waitFor(() => expect(result.current.isLoaded).toBe(true));
    expect(result.current.gamesById.get('game-1')?.phase).toBe('Ready');
  });

  it('keeps preparation progress scoped to both game and session', async () => {
    getStatus.mockResolvedValue([
      {
        session_id: 'session-1',
        completed_games: 0,
        total_games: 2,
        games: [
          { game_id: 'game-1', phase: 'Preparing' },
          { game_id: 'game-2', phase: 'Preparing' },
        ],
      },
    ]);
    const { result } = renderHook(() => useBackgroundIndexingStatus());
    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    for (const gameId of ['game-1', 'game-2']) {
      emit<OnboardingIndexingSnapshotProgress>('onboarding_indexing:snapshot_progress', {
        session_id: 'session-1',
        game_id: gameId,
        phase: 'Classifying',
        completed_games: 0,
        total_games: 2,
        completed_roots: gameId === 'game-1' ? 2 : 5,
        total_roots: 10,
        folders_classified: 20,
        current_root: null,
        elapsed_ms: 100,
      });
    }

    expect(result.current.snapshotProgressByGame.get('game-1')?.completed_roots).toBe(2);
    expect(result.current.snapshotProgressByGame.get('game-2')?.completed_roots).toBe(5);
  });

  it('removes progress when its indexing session is no longer present', async () => {
    getStatus
      .mockResolvedValueOnce([
        {
          session_id: 'session-1',
          completed_games: 0,
          total_games: 1,
          games: [{ game_id: 'game-1', phase: 'Preparing' }],
        },
      ])
      .mockResolvedValueOnce([]);
    const { result } = renderHook(() => useBackgroundIndexingStatus());
    await waitFor(() => expect(result.current.sessions).toHaveLength(1));
    emit<OnboardingIndexingSnapshotProgress>('onboarding_indexing:snapshot_progress', {
      session_id: 'session-1',
      game_id: 'game-1',
      phase: 'Classifying',
      completed_games: 0,
      total_games: 1,
      completed_roots: 2,
      total_roots: 10,
      folders_classified: 20,
      current_root: null,
      elapsed_ms: 100,
    });
    expect(result.current.snapshotProgressByGame.has('game-1')).toBe(true);

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.sessions).toHaveLength(0);
    expect(result.current.snapshotProgressByGame.has('game-1')).toBe(false);
  });
});
