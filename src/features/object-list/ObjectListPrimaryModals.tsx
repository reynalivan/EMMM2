import ObjectListModals, { SYNC_CONFIRM_RESET } from './ObjectListModals';
import type { GameConfig } from '../../types/game';
import type { WorkspaceObjectNode } from '../../types/workspace';
import type { useObjectListLogic } from './useObjectListLogic';

type ObjectListModalsState = ReturnType<typeof useObjectListLogic>['modals'];
type ObjectListHandlers = ReturnType<typeof useObjectListLogic>['handlers'];

interface ObjectListPrimaryModalsProps {
  activeGame: GameConfig | null;
  objects: WorkspaceObjectNode[];
  modals: ObjectListModalsState;
  handlers: ObjectListHandlers;
  createModalOpen: boolean;
  pendingPaths: string[] | null;
  autoSetupOpen: boolean;
  onCloseCreate: () => void;
  onCloseAutoSetup: () => void;
}

export default function ObjectListPrimaryModals({
  activeGame,
  objects,
  modals,
  handlers,
  createModalOpen,
  pendingPaths,
  autoSetupOpen,
  onCloseCreate,
  onCloseAutoSetup,
}: ObjectListPrimaryModalsProps) {
  return (
    <ObjectListModals
      activeGame={activeGame}
      editObject={modals.editObject}
      onCloseEdit={() => modals.setEditObject(null)}
      syncConfirm={modals.syncConfirm}
      onApplySyncMatch={handlers.handleApplySyncMatch}
      onEditManually={() => {
        const object = objects.find((candidate) => candidate.id === modals.syncConfirm.objectId);
        modals.setSyncConfirm(SYNC_CONFIRM_RESET);
        if (object) {
          modals.setEditObject(object);
        }
      }}
      onCloseSyncConfirm={() => modals.setSyncConfirm(SYNC_CONFIRM_RESET)}
      scanReview={modals.scanReview}
      onCommitScan={handlers.handleCommitScan}
      onCloseScanReview={handlers.handleCloseScanReview}
      createModalOpen={createModalOpen}
      pendingPaths={pendingPaths}
      onImportDropped={async (newObjId, newObjName, paths) => {
        await handlers.handleDropOnNewObjectSubmit(newObjId, newObjName, paths);
        onCloseCreate();
      }}
      onCloseCreate={onCloseCreate}
      autoSetupOpen={autoSetupOpen}
      onCloseAutoSetup={onCloseAutoSetup}
      deleteObjectDialog={modals.deleteObjectDialog}
      onConfirmDeleteObject={handlers.confirmDeleteObject}
      onCancelDeleteObject={() => modals.setDeleteObjectDialog({ open: false, id: '', name: '' })}
      forceDeleteObjectDialog={modals.forceDeleteObjectDialog}
      onConfirmForceDeleteObject={handlers.confirmForceDeleteObject}
      onCancelForceDeleteObject={() =>
        modals.setForceDeleteObjectDialog({ open: false, id: '', name: '', count: 0 })
      }
      mismatchConfirm={modals.mismatchConfirm}
      onConfirmMismatchHandler={() => {
        if (modals.mismatchConfirm) {
          handlers.handleDropAutoOrganize(modals.mismatchConfirm);
        }
        modals.setMismatchConfirm(null);
      }}
      onCancelMismatchHandler={() => modals.setMismatchConfirm(null)}
    />
  );
}
