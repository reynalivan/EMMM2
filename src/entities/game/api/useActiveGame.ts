import { useQuery } from '@tanstack/react-query';
import { commands } from '@/shared/api/tauri/bindings';

export function useActiveGame() {
  // ponytail: read the settings query directly rather than useSettings() — that
  // hook also builds 9 mutation objects this caller never touches, and it is
  // mounted from ~34 files.
  const {
    data: settings,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['settings'],
    queryFn: () => commands.getSettings(),
  });

  const games = settings?.games || [];
  const activeGameId = settings?.active_game_id ?? null;
  const activeGame = games.find((g) => g.id === activeGameId) || null;

  return {
    activeGame,
    isLoading,
    error,
    games,
  };
}
