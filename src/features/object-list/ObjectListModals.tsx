/**
 * ObjectListModals — all modal/dialog components used by ObjectList.
 * Extracted from ObjectList for modularity (350-line limit).
 */

import ConfirmDialog from '../../components/ui/ConfirmDialog';
import EditObjectModal from './EditObjectModal';
import SyncConfirmModal from './SyncConfirmModal';
import CreateObjectModal from './CreateObjectModal';
import ScanReviewModal from './ScanReviewModal';
import AutoSetupModal from './AutoSetupModal';
import { useTranslation } from 'react-i18next';
import type { ObjectSummary } from '../../types/object';
import type { MatchedDbEntry } from '../../lib/bindings';
import type { ScanPreviewItem, ConfirmedScanItem } from '../../lib/services/scanService';
import type { MasterDbEntry } from './scanReviewHelpers';
import type { GameConfig } from '../../types/game';

interface SyncConfirmState {
  open: boolean;
  objectId: string;
  objectName: string;
  itemType: 'object' | 'folder';
  match: MatchedDbEntry | null;
  isLoading: boolean;
  currentData: {
    name: string;
    object_type: string;
    metadata: Record<string, unknown> | null;
    thumbnail_path: string | null;
  } | null;
}

const SYNC_CONFIRM_RESET: SyncConfirmState = {
  open: false,
  objectId: '',
  objectName: '',
  itemType: 'object',
  match: null,
  isLoading: false,
  currentData: null,
};

interface ModalsProps {
  activeGame: GameConfig | null;
  /* Edit modal */
  editObject: ObjectSummary | null;
  onCloseEdit: () => void;
  /* Sync confirm modal */
  syncConfirm: SyncConfirmState;
  onApplySyncMatch: (match: MatchedDbEntry) => void;
  onEditManually: () => void;
  onCloseSyncConfirm: () => void;
  /* Scan review modal */
  scanReview: {
    open: boolean;
    items: ScanPreviewItem[];
    masterDbEntries: MasterDbEntry[];
    isCommitting: boolean;
  };
  onCommitScan: (items: ConfirmedScanItem[]) => void;
  onCloseScanReview: () => void;
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
  /* Mismatch Auto-Organize confirm */
  mismatchConfirm: string[] | null;
  onConfirmMismatchHandler: () => void;
  onCancelMismatchHandler: () => void;
}

export default function ObjectListModals({
  activeGame,
  editObject,
  onCloseEdit,
  syncConfirm,
  onApplySyncMatch,
  onEditManually,
  onCloseSyncConfirm,
  scanReview,
  onCommitScan,
  onCloseScanReview,
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
  mismatchConfirm,
  onConfirmMismatchHandler,
  onCancelMismatchHandler,
}: ModalsProps) {
  const { t } = useTranslation(['objects', 'common']);

  return (
    <>
      {/* Edit Object Modal (US-3.3) */}
      <EditObjectModal open={!!editObject} object={editObject} onClose={onCloseEdit} />

      {/* Sync Confirm Modal (single-object DB match) */}
      <SyncConfirmModal
        open={syncConfirm.open}
        objectName={syncConfirm.objectName}
        currentData={syncConfirm.currentData}
        match={syncConfirm.match}
        isLoading={syncConfirm.isLoading}
        onApply={onApplySyncMatch}
        onEditManually={onEditManually}
        onClose={onCloseSyncConfirm}
      />

      {/* Scan Review Modal (bulk scan results — US-2.3) */}
      <ScanReviewModal
        activeGame={activeGame}
        open={scanReview.open}
        items={scanReview.items}
        masterDbEntries={scanReview.masterDbEntries}
        isCommitting={scanReview.isCommitting}
        onConfirm={onCommitScan}
        onClose={onCloseScanReview}
      />

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

      {/* Mismatch Auto-Organize confirmation dialog */}
      <ConfirmDialog
        open={!!mismatchConfirm}
        title={t('auto_organize.dialog_title')}
        message={t('auto_organize.dialog_message', { count: mismatchConfirm?.length || 0 })}
        confirmLabel={t('auto_organize.confirm')}
        cancelLabel={t('common:actions.cancel')}
        onConfirm={onConfirmMismatchHandler}
        onCancel={onCancelMismatchHandler}
      />
    </>
  );
}

export { SYNC_CONFIRM_RESET };
export type { SyncConfirmState };
