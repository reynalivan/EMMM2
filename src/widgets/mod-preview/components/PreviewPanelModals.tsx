import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '@/entities/game-object';
import type { DuplicateInfo } from '@/entities/workspace';
import ConfirmDialog from '../../../shared/ui/components/ui/ConfirmDialog';
import DuplicateWarningModal from './DuplicateWarningModal';

interface PreviewPanelModalsProps {
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
  handleDuplicateForceEnable: (ignoreFuture: boolean) => void;
  handleDuplicateEnableOnly: () => void;
  handleDuplicateCancel: () => void;
}

export default function PreviewPanelModals({
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
}: PreviewPanelModalsProps) {
  const { t } = useTranslation(['preview', 'common']);

  // Local state for Rename input
  const [renameInput, setRenameInput] = useState('');

  useEffect(() => {
    if (renameDialog.open && renameDialog.folder) {
      setTimeout(() => setRenameInput(renameDialog.folder!.name), 0);
    }
  }, [renameDialog.open, renameDialog.folder]);

  return (
    <>
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
          <button>{t('common:actions.close')}</button>
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
    </>
  );
}
