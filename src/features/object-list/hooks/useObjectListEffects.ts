import { useEffect } from 'react';
import { subscribeWorkspaceIntent } from '../../workspace-runtime/workspaceIntentBus';

interface UseObjectListEffectsOptions {
  activeGameId: string | null;
  handleBackgroundSync: () => Promise<void>;
  handleDropAutoOrganize: (paths: string[]) => void;
  handleArchivesInteractively: (
    archives: string[],
    options: {
      type: 'item';
      pathsToIngest: string[];
      targetFolder: string;
      targetObjectId: string;
    },
  ) => void;
}

export function useObjectListEffects({
  activeGameId,
  handleBackgroundSync,
  handleDropAutoOrganize,
  handleArchivesInteractively,
}: UseObjectListEffectsOptions): void {
  useEffect(() => {
    if (!activeGameId) {
      return;
    }

    void handleBackgroundSync();
  }, [activeGameId, handleBackgroundSync]);

  useEffect(() => {
    return subscribeWorkspaceIntent((intent) => {
      if (intent.type === 'autoOrganizePaths') {
        handleDropAutoOrganize(intent.paths);
        return;
      }

      handleArchivesInteractively(intent.archives, {
        type: 'item',
        pathsToIngest: intent.nonArchivePaths,
        targetFolder: intent.targetDir,
        targetObjectId: '',
      });
    });
  }, [handleArchivesInteractively, handleDropAutoOrganize]);
}
