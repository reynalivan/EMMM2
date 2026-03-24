import { useQuery } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import type { ActiveKeyBinding } from '../../../types/settings';
import { useActiveGame } from '../../../hooks/useActiveGame';

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
    queryFn: () => commands.getActiveKeybindings({ gameId: activeGame!.id }),
    enabled: !!activeGame?.id,
    staleTime: 60_000,
  });

  return {
    keybindings: query.data ?? [],
    isLoading: query.isLoading,
    isError: query.isError,
  };
}
