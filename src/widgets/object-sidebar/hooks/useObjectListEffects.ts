import { useEffect } from 'react';
import { subscribeWorkspaceIntent } from '@/features/workspace-runtime';

interface UseObjectListEffectsOptions {
  activeGameId: string | null;
  handleBackgroundSync: () => Promise<void>;
  handleDropAutoOrganize: (paths: string[]) => void;
}

export function useObjectListEffects({
  activeGameId,
  handleBackgroundSync,
  handleDropAutoOrganize,
}: UseObjectListEffectsOptions): void {
  useEffect(() => {
    if (!activeGameId) {
      return;
    }

    void handleBackgroundSync();
  }, [activeGameId, handleBackgroundSync]);

  useEffect(() => {
    return subscribeWorkspaceIntent((intent) => {
      handleDropAutoOrganize(intent.paths);
    });
  }, [handleDropAutoOrganize]);
}
