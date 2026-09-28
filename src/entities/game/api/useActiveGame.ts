import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { gameGateway } from './gameGateway';

export function useActiveGame() {
  const activeGameId = useAppStore((state) => state.activeGameId);
  // ponytail: read the settings query directly rather than useSettings() — that
  // hook also builds 9 mutation objects this caller never touches, and it is
  // mounted from ~34 files.
  const {
    data: settings,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['settings'],
    queryFn: () => gameGateway.getSettings(),
  });

  const games = settings?.games || [];
  const activeGame = games.find((g) => g.id === activeGameId) || null;

  return {
    activeGame,
    isLoading,
    error,
    games,
  };
}
