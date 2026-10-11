import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '@/app/store';
import { demoDashboardGateway } from '../../../demo/dashboard';
import { demoGameSettings } from '../../../demo/game';
import { createWrapper } from '../../../tests/testing/test-utils';
import { useActiveKeybindings } from './useActiveKeybindings';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));
vi.mock('@/shared/lib/appMode', () => ({ isDemoMode: true }));
vi.mock('@/demo/dashboard', async () => await import('../../../demo/dashboard'));
vi.mock('@/entities/game', async () => {
  const { demoGameSettings } = await import('../../../demo/game');
  return {
    useActiveGame: () => ({
      activeGame: demoGameSettings.games[0],
      games: demoGameSettings.games,
      isLoading: false,
      error: null,
    }),
  };
});

describe('demo active keybindings', () => {
  it('loads fixture bindings without waiting for an activation that demo never starts', async () => {
    vi.clearAllMocks();
    const gameId = demoGameSettings.games[0].id;
    useAppStore.setState({
      activeGameId: gameId,
      requestedGameId: null,
      gameActivationByGame: {},
      diskReconcileByGame: {},
    });
    const expected = await demoDashboardGateway.getActiveKeybindings(gameId);
    expect(expected.length).toBeGreaterThan(0);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper });
    await waitFor(() => expect(result.current.keybindings).toEqual(expected));
    expect(result.current.isLoading).toBe(false);
    expect(invoke).not.toHaveBeenCalled();
  });
});
