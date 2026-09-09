/**
 * ObjectListModals — all modal/dialog components used by ObjectList.
 * Extracted from ObjectList for modularity (350-line limit).
 */

import ConfirmDialog from '../../../shared/ui/components/ui/ConfirmDialog';
import EditObjectModal from './EditObjectModal';
import CreateObjectModal from './CreateObjectModal';
import AutoSetupModal from './AutoSetupModal';
import { useTranslation } from 'react-i18next';
import type { ObjectSummary } from '@/entities/game-object';

interface ModalsProps {
  /* Edit modal */
  editObject: ObjectSummary | null;
  onCloseEdit: () => void;
  /* Create modal */
  createModalOpen: boolean;
  pendingPaths?: string[] | null;
  onImportDropped?: (newObjectId: string, objectName: string, paths: string[]) => void;
  onCloseCreate: () => void;
  /* Auto Setup modal */
  autoSetupOpen: boolean;
  onCloseAutoSetup: () => void;
  /* Delete Object dialog */
  deleteObjectDialog: { open: boolean; id: string; name: string };
  onConfirmDeleteObject: () => void;
  onCancelDeleteObject: () => void;
  /* Force Delete Object dialog */
  forceDeleteObjectDialog: { open: boolean; id: string; name: string; count: number };
  onConfirmForceDeleteObject: () => void;
  onCancelForceDeleteObject: () => void;
}

export default function ObjectListModals({
  editObject,
  onCloseEdit,
  createModalOpen,
  pendingPaths,
  onImportDropped,
  onCloseCreate,
  autoSetupOpen,
  onCloseAutoSetup,
  deleteObjectDialog,
  onConfirmDeleteObject,
  onCancelDeleteObject,
  forceDeleteObjectDialog,
  onConfirmForceDeleteObject,
  onCancelForceDeleteObject,
}: ModalsProps) {
  const { t } = useTranslation(['objects', 'common']);

  return (
    <>
      {/* Edit Object Modal (US-3.3) */}
      <EditObjectModal open={!!editObject} object={editObject} onClose={onCloseEdit} />

      {/* Create Object Modal (US-3.3) */}
      <CreateObjectModal
        open={createModalOpen}
        onClose={onCloseCreate}
        pendingPaths={pendingPaths}
        onImportDropped={onImportDropped}
      />

      {/* Auto Setup Modal */}
      <AutoSetupModal open={autoSetupOpen} onClose={onCloseAutoSetup} />

      {/* Delete Object confirmation dialog */}
      <ConfirmDialog
        open={deleteObjectDialog.open}
        title={t('delete_dialog.title_object')}
        message={t('delete_dialog.message_object', { name: deleteObjectDialog.name })}
        confirmLabel={t('delete_dialog.confirm')}
        cancelLabel={t('common:actions.cancel')}
        danger
        onConfirm={onConfirmDeleteObject}
        onCancel={onCancelDeleteObject}
      />

      {/* Secondary confirmation dialog when object contains mods */}
      <ConfirmDialog
        open={forceDeleteObjectDialog.open}
        title={t('delete_dialog.title_mods')}
        message={t('delete_dialog.message_mods', {
          name: forceDeleteObjectDialog.name,
          count: forceDeleteObjectDialog.count,
          suffix: forceDeleteObjectDialog.count === 1 ? '' : 's',
        })}
        confirmLabel={t('delete_dialog.confirm_mods', {
          count: forceDeleteObjectDialog.count,
          suffix: forceDeleteObjectDialog.count === 1 ? '' : 's',
        })}
        cancelLabel={t('common:actions.cancel')}
        danger
        onConfirm={onConfirmForceDeleteObject}
        onCancel={onCancelForceDeleteObject}
      />
    </>
  );
}
