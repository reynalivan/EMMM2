import ObjectListModals from './ObjectListModals';
import type { useObjectListLogic } from '../hooks/useObjectListLogic';

type ObjectListModalsState = ReturnType<typeof useObjectListLogic>['modals'];
type ObjectListHandlers = ReturnType<typeof useObjectListLogic>['handlers'];

interface ObjectListPrimaryModalsProps {
  modals: ObjectListModalsState;
  handlers: ObjectListHandlers;
  createModalOpen: boolean;
  pendingPaths: string[] | null;
  autoSetupOpen: boolean;
  onCloseCreate: () => void;
  onCloseAutoSetup: () => void;
}

export default function ObjectListPrimaryModals({
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
      editObject={modals.editObject}
      onCloseEdit={() => modals.setEditObject(null)}
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
    />
  );
}
