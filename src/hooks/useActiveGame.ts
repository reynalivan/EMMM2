import { useSettings } from './useSettings';
import { useAppStore } from '../stores/useAppStore';

export function useActiveGame() {
  const { activeGameId } = useAppStore();
  const { settings, isLoading, error } = useSettings();

  const games = settings?.games || [];
  const activeGame = games.find((g) => g.id === activeGameId) || null;

  return {
    activeGame,
    isLoading,
    error,
    games,
  };
}
