import type { ModFolder } from '../../types/mod';
import type { ObjectSummary } from '../../types/object';
import MoveToObjectDialog from './MoveToObjectDialog';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import DuplicateWarningModal, { type DuplicateInfo } from './DuplicateWarningModal';
import { BulkTagModal } from './BulkTagModal';
import PinEntryModal from '../safe-mode/PinEntryModal';

export interface FolderGridModalsProps {
  moveDialog: { open: boolean; folder: ModFolder | null };
  closeMoveDialog: () => void;
  objects: ObjectSummary[];
  handleMoveToObject: (
    folder: ModFolder,
    targetId: string,
    status: 'disabled' | 'only-enable' | 'keep',
  ) => void;
  deleteConfirm: { open: boolean; folder: ModFolder | null };
  setDeleteConfirm: (state: { open: boolean; folder: ModFolder | null }) => void;
  handleDeleteConfirm: () => void;
  bulkDeleteConfirm: boolean;
  setBulkDeleteConfirm: (open: boolean) => void;
  handleBulkDeleteConfirm: () => void;
  bulkTagOpen: boolean;
  setBulkTagOpen: (open: boolean) => void;
  gridSelection: Set<string>;
  duplicateWarning: { open: boolean; folder: ModFolder | null; duplicates: DuplicateInfo[] };
  handleDuplicateForceEnable: () => void;
  handleDuplicateEnableOnly: () => void;
  handleDuplicateCancel: () => void;
  pinSafeDialog: { open: boolean };
  handleToggleSafeCancel: () => void;
  handleToggleSafeSubmit: () => void;
}

export default function FolderGridModals({
  moveDialog,
  closeMoveDialog,
  objects,
  handleMoveToObject,
  deleteConfirm,
  setDeleteConfirm,
  handleDeleteConfirm,
  bulkDeleteConfirm,
  setBulkDeleteConfirm,
  handleBulkDeleteConfirm,
  bulkTagOpen,
  setBulkTagOpen,
  gridSelection,
  duplicateWarning,
  handleDuplicateForceEnable,
  handleDuplicateEnableOnly,
  handleDuplicateCancel,
  pinSafeDialog,
  handleToggleSafeCancel,
  handleToggleSafeSubmit,
}: FolderGridModalsProps) {
  return (
    <>
      {/* Move To Object Dialog */}
      {moveDialog.open && moveDialog.folder && (
        <MoveToObjectDialog
          open={moveDialog.open}
          onClose={closeMoveDialog}
          objects={objects}
          currentObjectId={moveDialog.folder.object_id ?? undefined}
          currentStatus={moveDialog.folder.is_enabled}
          onSubmit={(targetId, status) => {
            if (!moveDialog.folder) return;
            handleMoveToObject(moveDialog.folder, targetId, status);
            closeMoveDialog();
          }}
        />
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={deleteConfirm.open}
        title="Delete to Trash?"
        message={`Are you sure you want to move "${deleteConfirm.folder?.name}" to trash? You can undo this later.`}
        confirmLabel="Move to Trash"
        danger
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteConfirm({ open: false, folder: null })}
      />

      <ConfirmDialog
        open={bulkDeleteConfirm}
        title={`Delete ${gridSelection.size} Mods?`}
        message={`Are you sure you want to move ${gridSelection.size} mods to trash?`}
        confirmLabel="Move All to Trash"
        danger
        onConfirm={handleBulkDeleteConfirm}
        onCancel={() => setBulkDeleteConfirm(false)}
      />

      {bulkTagOpen && (
        <BulkTagModal
          isOpen={true}
          onClose={() => setBulkTagOpen(false)}
          selectedPaths={Array.from(gridSelection)}
        />
      )}

      {/* Duplicate Character Warning */}
      <DuplicateWarningModal
        open={duplicateWarning.open}
        targetName={duplicateWarning.folder?.name ?? ''}
        duplicates={duplicateWarning.duplicates}
        onForceEnable={handleDuplicateForceEnable}
        onEnableOnlyThis={handleDuplicateEnableOnly}
        onCancel={handleDuplicateCancel}
      />

      <PinEntryModal
        open={pinSafeDialog.open}
        onClose={handleToggleSafeCancel}
        onSuccess={handleToggleSafeSubmit}
      />
    </>
  );
}
