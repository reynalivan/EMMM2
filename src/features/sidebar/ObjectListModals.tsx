/**
 * ObjectListModals — all modal/dialog components used by ObjectList.
 * Extracted from ObjectList for modularity (350-line limit).
 */

import ConfirmDialog from '../../components/ui/ConfirmDialog';
import EditObjectModal from './EditObjectModal';
import SyncConfirmModal from './SyncConfirmModal';
import CreateObjectModal from './CreateObjectModal';
import ScanReviewModal from './ScanReviewModal';
import type { ObjectSummary } from '../../types/object';
import type { MatchedDbEntry } from './SyncConfirmModal';
import type { ScanPreviewItem, ConfirmedScanItem } from '../../services/scanService';
import type { MasterDbEntry } from './ScanReviewModal';
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
  /* Delete dialog */
  deleteDialog: { open: boolean; path: string; name: string; itemCount: number };
  onConfirmDelete: () => void;
  onCancelDelete: () => void;
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
}

export default function ObjectListModals({
  activeGame,
  deleteDialog,
  onConfirmDelete,
  onCancelDelete,
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
}: ModalsProps) {
  return (
    <>
      {/* NC-3.3-02: Delete confirmation dialog */}
      <ConfirmDialog
        open={deleteDialog.open}
        title="Delete Folder?"
        message={`"${deleteDialog.name}" contains ${deleteDialog.itemCount} item${deleteDialog.itemCount !== 1 ? 's' : ''}. This will move everything to trash.`}
        confirmLabel="Move to Trash"
        cancelLabel="Cancel"
        danger
        onConfirm={onConfirmDelete}
        onCancel={onCancelDelete}
      />

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
    </>
  );
}

export { SYNC_CONFIRM_RESET };
export type { SyncConfirmState };
