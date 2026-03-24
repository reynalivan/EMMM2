/**
 * useCorridor — Single query for corridor state.
 *
 * Replaces: useCorridorRuntimeSnapshot + useWorkspaceContext + resolveActiveCollection chain.
 * Backend computes is_dirty, active_  const activeGame = games?.find((g: any) => g.id === activeGameId);
 */

import { useQuery, keepPreviousData } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import type { CorridorSnapshot } from '../../../types/collection';
import { corridorKeys } from '../queryKeys';

export function useCorridor(gameId: string | null, isSafe: boolean) {
  return useQuery<CorridorSnapshot>({
    queryKey: corridorKeys.state(gameId ?? '', isSafe),
    queryFn: () =>
      commands.getCorridorState({
        gameId: gameId ?? '',
        isSafe,
      }),
    enabled: !!gameId,
    staleTime: 5_000,
    placeholderData: keepPreviousData,
  });
}
