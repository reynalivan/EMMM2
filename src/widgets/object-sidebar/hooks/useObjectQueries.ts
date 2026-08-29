import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import { useAppStore } from '../../../app/store/useAppStore';
import { useActiveGame } from '@/pages/dashboard/hooks/useActiveGame';
import { getCategoryCounts } from '../services/objectService';
import {
  buildObjectListRefreshDescriptor,
  objectKeys,
  type CategoryCount,
  type GameSchema,
} from './objectQueryCache';
import { publishRuntimeDescriptor } from '@/features/runtime-sync/queryRefresh';
import type { GameType } from '@/entities/game/model/game';

export function useCategoryCounts() {
  const { activeGame } = useActiveGame();
  const gameId = activeGame?.id ?? '';

  return useQuery<CategoryCount[]>({
    queryKey: objectKeys.counts(gameId),
    queryFn: () => getCategoryCounts(gameId),
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
    queryFn: () => commands.getGameSchema(gameType!),
    enabled: gameType !== undefined,
    staleTime: Infinity,
  });
}

export function useGameSwitch() {
  const setActiveGameId = useAppStore((state) => state.setActiveGameId);
  const queryClient = useQueryClient();

  const switchGame = async (gameId: string) => {
    await setActiveGameId(gameId);
    await publishRuntimeDescriptor(
      queryClient,
      buildObjectListRefreshDescriptor({
        includeFolders: true,
        includeCollections: true,
        includeRuntime: true,
        includeDashboard: true,
      }),
      'active',
    );
  };

  return { switchGame };
}
import type { DbEntry } from '../../../shared/api/tauri/bindings.gen';

export function useMasterDb() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type;

  return useQuery<DbEntry[]>({
    queryKey: ['master-db', gameType],
    queryFn: () => commands.getMasterDb(gameType!),
    enabled: !!gameType,
    staleTime: Infinity,
  });
}
