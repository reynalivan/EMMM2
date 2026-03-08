import { useState, useEffect } from 'react';
import type { ModFolder } from '../../../types/mod';
import MoveToObjectDialog from '../../folder-grid/MoveToObjectDialog';
import ConfirmDialog from '../../../components/ui/ConfirmDialog';
import DuplicateWarningModal, { type DuplicateInfo } from '../../folder-grid/DuplicateWarningModal';
import PinEntryModal from '../../safe-mode/PinEntryModal';
import { useObjects } from '../../../hooks/useObjects';

interface PreviewPanelModalsProps {
  // Move Dialog
  moveDialog: { open: boolean; folder: ModFolder | null };
  closeMoveDialog: () => void;
  handleMoveToObject: (
    folder: ModFolder,
    targetId: string,
    status: 'disabled' | 'only-enable' | 'keep',
  ) => void;

  // Delete Dialog
  deleteConfirm: { open: boolean; folder: ModFolder | null };
  setDeleteConfirm: (state: { open: boolean; folder: ModFolder | null }) => void;
  handleDeleteConfirm: () => void;

  // Rename Dialog
  renameDialog: { open: boolean; folder: ModFolder | null };
  handleRenameCancel: () => void;
  handleRenameSubmit: (newName: string) => void;

  // Duplicate Warning
  duplicateWarning: { open: boolean; folder: ModFolder | null; duplicates: DuplicateInfo[] };
  handleDuplicateForceEnable: () => void;
  handleDuplicateEnableOnly: () => void;
  handleDuplicateCancel: () => void;

  // Pin Safe
  pinSafeDialog: { open: boolean; folder: ModFolder | null };
  handleToggleSafeCancel: () => void;
  handleToggleSafeSubmit: () => void;
}

export default function PreviewPanelModals({
  moveDialog,
  closeMoveDialog,
  handleMoveToObject,
  deleteConfirm,
  setDeleteConfirm,
  handleDeleteConfirm,
  renameDialog,
  handleRenameCancel,
  handleRenameSubmit,
  duplicateWarning,
  handleDuplicateForceEnable,
  handleDuplicateEnableOnly,
  handleDuplicateCancel,
  pinSafeDialog,
  handleToggleSafeCancel,
  handleToggleSafeSubmit,
}: PreviewPanelModalsProps) {
  // We need to fetch objects for the MoveToObjectDialog
  const { data: objectsData } = useObjects();
  const objects = objectsData || [];

  // Local state for Rename input
  const [renameInput, setRenameInput] = useState('');

  useEffect(() => {
    if (renameDialog.open && renameDialog.folder) {
      setTimeout(() => setRenameInput(renameDialog.folder!.name), 0);
    }
  }, [renameDialog.open, renameDialog.folder]);

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

      {/* Rename Dialog */}
      <dialog className={`modal ${renameDialog.open ? 'modal-open' : ''}`}>
        <div className="modal-box">
          <h3 className="font-bold text-lg">Rename Folder</h3>
          <p className="py-4 text-sm opacity-80">
            Enter a new folder name for "{renameDialog.folder?.name}".
          </p>
          <input
            type="text"
            className="input input-bordered w-full"
            value={renameInput}
            onChange={(e) => setRenameInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleRenameSubmit(renameInput);
            }}
            autoFocus
          />
          <div className="modal-action">
            <button className="btn" onClick={handleRenameCancel}>
              Cancel
            </button>
            <button
              className="btn btn-primary"
              onClick={() => handleRenameSubmit(renameInput)}
              disabled={!renameInput.trim() || renameInput === renameDialog.folder?.name}
            >
              Rename
            </button>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop" onClick={handleRenameCancel}>
          <button>close</button>
        </form>
      </dialog>

      {/* Duplicate Character Warning */}
      <DuplicateWarningModal
        open={duplicateWarning.open}
        targetName={duplicateWarning.folder?.name ?? ''}
        duplicates={duplicateWarning.duplicates}
        onForceEnable={handleDuplicateForceEnable}
        onEnableOnlyThis={handleDuplicateEnableOnly}
        onCancel={handleDuplicateCancel}
      />

      {/* Safe Mode Pin Entry */}
      <PinEntryModal
        open={pinSafeDialog.open}
        onClose={handleToggleSafeCancel}
        onSuccess={handleToggleSafeSubmit}
      />
    </>
  );
}
