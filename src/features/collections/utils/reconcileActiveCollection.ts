import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../../stores/useAppStore';
import { queryClient } from '../../../lib/queryClient';
import { invalidateCorridorRuntime } from './invalidateCorridorRuntime';

type AppStoreState = {
  activeGameId: string | null;
  safeMode: boolean;
};

type ReconcileDeps = {
  getState?: () => AppStoreState;
  invokeFn?: typeof invoke;
};

interface ReconcileOptions {
  gameId?: string | null;
  safeMode?: boolean;
}

export async function reconcileActiveCollection(
  options: ReconcileOptions = {},
  deps: ReconcileDeps = {},
) {
  const getState = deps.getState ?? (useAppStore.getState as () => AppStoreState);
  const invokeFn = deps.invokeFn ?? invoke;
  const state = getState();

  const gameId = options.gameId ?? state.activeGameId;
  const safeMode = options.safeMode ?? state.safeMode;

  if (!gameId) {
    return false;
  }

  try {
    const reconciledCount = await invokeFn<number>('reconcile_current_corridor', {
      gameId,
      isSafe: safeMode,
    });
    await invalidateCorridorRuntime(queryClient);
    return reconciledCount > 0;
  } catch (error) {
    console.warn('Failed to reconcile corridor runtime after refresh:', error);
    return false;
  }
}
