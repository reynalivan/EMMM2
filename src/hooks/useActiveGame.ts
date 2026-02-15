import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import type { GameConfig } from '../types/game';

export function useActiveGame() {
  const { activeGameId } = useAppStore();

  const {
    data: games,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['games'],
    queryFn: async () => {
      return await invoke<GameConfig[]>('get_games');
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });

  const activeGame = games?.find((g) => g.id === activeGameId);

  return {
    activeGame,
    isLoading,
    error,
    games,
  };
}
