import { useQuery } from '@tanstack/react-query';
import { settingsQueryOptions } from './settingsQuery';
import { useAppStore } from '../stores/useAppStore';
import type { AppSettings } from '../types/settings';

export function useActiveGame() {
  // ponytail: read the settings query directly rather than useSettings() — that
  // hook also builds 9 mutation objects this caller never touches, and it is
  // mounted from ~34 files.
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { data: settings, isLoading, error } = useQuery<AppSettings>(settingsQueryOptions);

  const games = settings?.games || [];
  const activeGame = games.find((g) => g.id === activeGameId) || null;

  return {
    activeGame,
    isLoading,
    error,
    games,
  };
}
