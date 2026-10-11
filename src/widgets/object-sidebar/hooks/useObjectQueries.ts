import { useQuery } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import { useActiveGame } from '@/entities/game';
import type { GameSchema } from '@/entities/game-object';
import { objectKeys } from '@/features/workspace-runtime';
import type { GameType } from '@/entities/game';

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

import type { DbEntry } from '../../../shared/api/tauri/bindings.gen';

export function useMasterDb() {
  const { activeGame } = useActiveGame();
  const gameType = activeGame?.game_type;

  return useQuery<DbEntry[]>({
    queryKey: ['master-db', gameType],
    queryFn: () => commands.getMasterDb(gameType!),
    enabled: gameType !== undefined,
    staleTime: Infinity,
  });
}
