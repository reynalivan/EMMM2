import { useQuery } from '@tanstack/react-query';
import type { ActiveKeyBinding } from '@/entities/settings';
import { useActiveGame } from '@/entities/game';
import { dashboardGateway } from '../api/dashboardGateway';

/**
 * TanStack Query hook for active keybindings.
 * Scans enabled mods' INI files for key bindings.
 * Only fetches when an active game is selected.
 * Longer staleTime (60s) since it touches the filesystem.
 */
export function useActiveKeybindings() {
  const { activeGame } = useActiveGame();

  const query = useQuery<ActiveKeyBinding[]>({
    queryKey: ['active-keybindings', activeGame?.id],
    queryFn: () => dashboardGateway.getActiveKeybindings(activeGame!.id),
    enabled: !!activeGame?.id,
    staleTime: 60_000,
  });

  return {
    keybindings: query.data ?? [],
    isLoading: query.isLoading,
    isError: query.isError,
  };
}
