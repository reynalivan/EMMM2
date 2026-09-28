import { useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import { buildObjectListRefreshDescriptor } from './objectMutationCache';

let latestSwitchSequence = 0;

export function useGameSwitch() {
  const setActiveGameId = useAppStore((state) => state.setActiveGameId);
  const queryClient = useQueryClient();

  const switchGame = async (gameId: string) => {
    const switchSequence = ++latestSwitchSequence;
    await setActiveGameId(gameId, { deferWorkspacePrefetch: true });
    const { activeGameId, requestedGameId } = useAppStore.getState();
    if (
      switchSequence !== latestSwitchSequence ||
      activeGameId !== gameId ||
      requestedGameId !== null
    ) {
      return;
    }
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
