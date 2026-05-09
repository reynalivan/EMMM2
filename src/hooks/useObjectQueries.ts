import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import { useAppStore } from '../stores/useAppStore';
import { useActiveGame } from './useActiveGame';
import { getCategoryCounts } from '../lib/services/objectService';
import {
  buildObjectListRefreshDescriptor,
  objectKeys,
  type CategoryCount,
  type GameSchema,
} from './objectQueryCache';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import type { GameType } from '../types/game';

export function useCategoryCounts() {
  const { activeGame } = useActiveGame();
  const { safeMode } = useAppStore();
  const gameId = activeGame?.id ?? '';

  return useQuery<CategoryCount[]>({
    queryKey: [...objectKeys.counts(gameId), safeMode],
    queryFn: () => getCategoryCounts(gameId, safeMode),
    enabled: !!gameId,
    staleTime: 30_000,
    refetchOnWindowFocus: false,
  });
}

export function useGameSchema() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type;

  return useQuery<GameSchema>({
    queryKey: objectKeys.schema(gameType as GameType),
    queryFn: () => commands.getGameSchema({ gameType: gameType! }),
    enabled: gameType !== undefined,
    staleTime: Infinity,
  });
}

export function useGameSwitch() {
  const { setActiveGameId } = useAppStore();
  const queryClient = useQueryClient();

  const switchGame = async (gameId: string) => {
    await setActiveGameId(gameId);
    await publishRuntimeDescriptor(
      queryClient,
      buildObjectListRefreshDescriptor({
        includeFolders: true,
        includeCollections: true,
        includeCorridor: true,
        includeDashboard: true,
      }),
      'active',
    );
  };

  return { switchGame };
}

export function useMasterDb() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type;

  return useQuery<string>({
    queryKey: ['master-db', gameType],
    queryFn: () => commands.getMasterDb({ gameType: gameType! }),
    enabled: !!gameType,
    staleTime: Infinity,
  });
}
