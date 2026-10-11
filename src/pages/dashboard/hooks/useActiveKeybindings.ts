import { useQuery } from '@tanstack/react-query';
import type { ActiveKeyBinding } from '@/entities/settings';
import { useActiveGame } from '@/entities/game';
import { useAppStore } from '@/app/store';
import { isDemoMode } from '@/shared/lib/appMode';
import { dashboardGateway } from '../api/dashboardGateway';

/**
 * TanStack Query hook for active keybindings.
 * Scans enabled mods' INI files for key bindings.
 * Waits for the existing core activation before starting optional disk reads.
 * Longer staleTime (60s) since it touches the filesystem.
 */
export function useActiveKeybindings() {
  const { activeGame, isLoading: gameLoading, error: gameError } = useActiveGame();
  const requestedGameId = useAppStore((state) => state.requestedGameId);
  const selectedGameId = useAppStore((state) => state.requestedGameId ?? state.activeGameId);
  const activation = useAppStore((state) =>
    selectedGameId ? state.gameActivationByGame[selectedGameId] : undefined,
  );
  const sourceUnavailable = useAppStore((state) =>
    selectedGameId ? state.diskReconcileByGame[selectedGameId]?.unavailable : null,
  );
  const failed =
    Boolean(gameError) ||
    (Boolean(selectedGameId) && !activeGame && !gameLoading && !requestedGameId) ||
    activation?.phase === 'failed' ||
    activation?.phase === 'source_unavailable' ||
    Boolean(sourceUnavailable);
  const enabled =
    Boolean(selectedGameId) &&
    !requestedGameId &&
    activeGame?.id === selectedGameId &&
    (isDemoMode || activation?.phase === 'ready') &&
    !failed;

  const query = useQuery<ActiveKeyBinding[]>({
    // A successor activation must not reuse or receive the previous baseline's scan.
    queryKey: ['active-keybindings', selectedGameId, activation?.generation],
    queryFn: () => {
      if (!selectedGameId) throw new Error('No game selected for keybinding scan');
      return dashboardGateway.getActiveKeybindings(selectedGameId);
    },
    enabled,
    staleTime: 60_000,
  });

  return {
    keybindings: enabled ? (query.data ?? []) : [],
    isLoading: Boolean(selectedGameId) && !failed && (!enabled || query.isPending),
    isError: failed || query.isError,
  };
}
