import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '../../../types/mod';
import MoveToObjectDialog from '../../folder-grid/MoveToObjectDialog';
import ConfirmDialog from '../../../components/ui/ConfirmDialog';
import DuplicateWarningModal, { type DuplicateInfo } from '../../folder-grid/DuplicateWarningModal';
import PinEntryModal from '../../safe-mode/PinEntryModal';

interface PreviewPanelModalsProps {
  // Move Dialog
  moveDialog: { open: boolean; folder: ModFolder | null };
  closeMoveDialog: () => void;
  handleMoveToObject: (
    folder: ModFolder,
    targetId: string,
    status: 'disabled' | 'only-enable' | 'keep',
  ) => void;
  objectId?: string;

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
  objectId,
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
  const { t } = useTranslation(['preview', 'common']);

  // Local state for Rename input
  const [renameInput, setRenameInput] = useState('');

  useEffect(() => {
    if (renameDialog.open && renameDialog.folder) {
      setTimeout(() => setRenameInput(renameDialog.folder!.name), 0);
    }
  }, [renameDialog.open, renameDialog.folder]);

  // currentPath from moveDialog.folder for the new MoveToObjectDialog props
  const currentPath = moveDialog.folder?.path ?? '';
  // Note: objectId is already in props, so we just use that directly or as defined below if shadowed

  return (
    <>
      {/* Move To Object Dialog */}
      {moveDialog.open && moveDialog.folder && (
        <MoveToObjectDialog
          isOpen={moveDialog.open}
          onClose={closeMoveDialog}
          targetModPaths={[currentPath]}
          currentObjectId={objectId || undefined}
          onSubmit={(targetId: string, status: 'disabled' | 'only-enable' | 'keep') => {
            if (!moveDialog.folder) return;
            handleMoveToObject(moveDialog.folder, targetId, status);
            closeMoveDialog();
          }}
        />
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={deleteConfirm.open}
        title={t('preview:modals.delete_title')}
        message={t('preview:modals.delete_message', { name: deleteConfirm.folder?.name })}
        confirmLabel={t('preview:modals.delete_confirm')}
        danger
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteConfirm({ open: false, folder: null })}
      />

      {/* Rename Dialog */}
      <dialog className={`modal ${renameDialog.open ? 'modal-open' : ''}`}>
        <div className="modal-box">
          <h3 className="font-bold text-lg">{t('preview:modals.rename_title')}</h3>
          <p className="py-4 text-sm opacity-80">
            {t('preview:modals.rename_message', { name: renameDialog.folder?.name })}
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
              {t('common:actions.cancel')}
            </button>
            <button
              className="btn btn-primary"
              onClick={() => handleRenameSubmit(renameInput)}
              disabled={!renameInput.trim() || renameInput === renameDialog.folder?.name}
            >
              {t('preview:actions.rename')}
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
