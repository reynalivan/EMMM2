/**
 * useCorridor — Single query for corridor state.
 *
 * Replaces the older corridor snapshot + active collection resolution chain.
 * Backend returns a corridor-level snapshot that already includes the active
 * collection and dirty-state semantics the UI needs.
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
