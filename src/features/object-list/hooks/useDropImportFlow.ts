import { useCallback } from 'react';
import { useActiveGame } from '../../dashboard/hooks/useActiveGame';
import { toast } from '../../../stores/useToastStore';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import { openImportBatchWizard } from '../../import-batches/launcher';

interface DropDeps {
  objects: WorkspaceObjectNode[];
}

export function useDropImportFlow({ objects }: DropDeps) {
  const { activeGame } = useActiveGame();

  const openSpecificImport = useCallback(
    (targetObjectId: string, paths: string[]) => {
      if (!activeGame || paths.length === 0) return;
      const target = objects.find((object) => object.id === targetObjectId);
      if (!target) {
        toast.error('Could not find target object.');
        return;
      }
      openImportBatchWizard({
        kind: 'sources',
        gameId: activeGame.id,
        flow: 'specific_import',
        targetMode: 'specific',
        targetObjectId,
        targetSubpath: null,
        paths,
      });
    },
    [activeGame, objects],
  );

  const handleDropOnItem = useCallback(
    async (objectId: string, paths: string[]) => openSpecificImport(objectId, paths),
    [openSpecificImport],
  );

  const handleDropAutoOrganize = useCallback(
    async (paths: string[]) => {
      if (!activeGame || paths.length === 0) return;
      openImportBatchWizard({
        kind: 'sources',
        gameId: activeGame.id,
        flow: 'auto_import',
        targetMode: 'auto',
        paths,
      });
    },
    [activeGame],
  );

  const handleDropOnNewObjectSubmit = useCallback(
    async (newObjectId: string, _objectName: string, paths: string[]) =>
      openSpecificImport(newObjectId, paths),
    [openSpecificImport],
  );

  return {
    handleDropOnItem,
    handleDropAutoOrganize,
    handleDropOnNewObjectSubmit,
  };
}
