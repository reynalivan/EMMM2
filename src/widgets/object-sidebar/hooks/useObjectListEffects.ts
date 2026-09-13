import { useEffect } from 'react';
import { subscribeWorkspaceIntent } from '@/features/workspace-runtime';
import { isDemoMode } from '@/shared/lib/appMode';

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
    if (!activeGameId || isDemoMode) {
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
