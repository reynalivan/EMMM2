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

  // Duplicate Warning
  duplicateWarning: {
    open: boolean;
    folder: Pick<ModFolder, 'id' | 'path' | 'name'> | null;
    duplicates: DuplicateInfo[];
  };
  handleDuplicateForceEnable: (ignoreFuture: boolean) => void;
  handleDuplicateEnableOnly: () => void;
  handleDuplicateCancel: () => void;
}

export default function PreviewPanelModals({
  deleteConfirm,
  setDeleteConfirm,
  handleDeleteConfirm,
  duplicateWarning,
  handleDuplicateForceEnable,
  handleDuplicateEnableOnly,
  handleDuplicateCancel,
}: PreviewPanelModalsProps) {
  const { t } = useTranslation('preview');

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
