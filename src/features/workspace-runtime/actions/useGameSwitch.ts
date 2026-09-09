import { useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import { buildObjectListRefreshDescriptor } from './objectMutationCache';

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
