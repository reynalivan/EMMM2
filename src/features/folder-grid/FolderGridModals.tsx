import type { ModFolder } from '../../types/mod';
import MoveToObjectDialog from './MoveToObjectDialog';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import { useAppStore } from '../../stores/useAppStore';
import ObjectConflictModal from './ObjectConflictModal';
import IgnoreManagementModal from './IgnoreManagementModal';
import { BulkTagModal } from './BulkTagModal';
import PinEntryModal from '../safe-mode/PinEntryModal';
import ActiveModContextDialog from './ActiveModContextDialog';
import SyncConfirmModal from '../object-list/SyncConfirmModal';
import type { MatchedDbEntry } from '../object-list/SyncConfirmModal';
import type { ObjectSummary } from '../../types/object';
import { useTranslation } from 'react-i18next';

export interface FolderGridModalsProps {
  moveDialog: { open: boolean; folder: ModFolder | null };
  closeMoveDialog: () => void;
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
  isIgnoreManagementOpen: boolean;
  setIsIgnoreManagementOpen: (open: boolean) => void;
  pinSafeDialog: { open: boolean };
  handleToggleSafeCancel: () => void;
  handleToggleSafeSubmit: () => void;
  activeContextDialog: { open: boolean; folder: ModFolder | null; isProcessing: boolean };
  handleActiveContextCancel: () => void;
  handleActiveContextSubmit: () => void;
  syncConfirm: {
    open: boolean;
    folder: ModFolder | null;
    match: MatchedDbEntry | null;
    isLoading: boolean;
    currentData: {
      name: string;
      object_type: string;
      metadata: Record<string, unknown> | null;
      thumbnail_path: string | null;
    } | null;
  };
  handleCloseSyncConfirm: () => void;
  handleApplySyncMatch: (match: MatchedDbEntry) => void;
  objectId?: string;
  currentPath?: string;
  objects: ObjectSummary[];
}

export default function FolderGridModals({
  moveDialog,
  closeMoveDialog,
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
  isIgnoreManagementOpen,
  setIsIgnoreManagementOpen,
  pinSafeDialog,
  handleToggleSafeCancel,
  handleToggleSafeSubmit,
  activeContextDialog,
  handleActiveContextCancel,
  handleActiveContextSubmit,
  syncConfirm,
  handleCloseSyncConfirm,
  handleApplySyncMatch,
  objectId,
  currentPath,
  objects,
}: FolderGridModalsProps) {
  const { t } = useTranslation(['grid']);

  return (
    <>
      {/* Move To Object Dialog */}
      {moveDialog.open && moveDialog.folder && (
        <MoveToObjectDialog
          isOpen={moveDialog.open}
          onClose={closeMoveDialog}
          targetModPaths={[currentPath ?? moveDialog.folder.path]} // Use currentPath if available, otherwise fallback
          currentObjectId={objectId || undefined}
          objects={objects}
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

      <ConfirmDialog
        open={bulkDeleteConfirm}
        title={t('modals.bulk_delete_title', { count: gridSelection.size })}
        message={t('modals.bulk_delete_msg', { count: gridSelection.size })}
        confirmLabel={t('modals.bulk_delete_confirm_btn')}
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

      {/* Conflict Resolution Modal (Structured Error) */}
      <ObjectConflictModal
        open={useAppStore((state) => state.duplicateConflictDialog.open)}
        folder={useAppStore((state) => state.duplicateConflictDialog.folder)}
        duplicates={useAppStore((state) => state.duplicateConflictDialog.duplicates)}
        onClose={() => useAppStore.getState().closeDuplicateConflictDialog()}
      />

      {/* Ignore Management Modal */}
      <IgnoreManagementModal
        open={isIgnoreManagementOpen}
        onClose={() => setIsIgnoreManagementOpen(false)}
      />

      <PinEntryModal
        open={pinSafeDialog.open}
        onClose={handleToggleSafeCancel}
        onSuccess={handleToggleSafeSubmit}
      />

      <ActiveModContextDialog
        key={activeContextDialog.folder?.path || 'dialog-hidden'}
        open={activeContextDialog.open}
        modName={activeContextDialog.folder?.name ?? ''}
        targetSafeStatus={!(activeContextDialog.folder?.is_safe ?? false)}
        isProcessing={activeContextDialog.isProcessing}
        onCancel={handleActiveContextCancel}
        onConfirm={handleActiveContextSubmit}
      />

      {/* Sync with DB Modal */}
      <SyncConfirmModal
        open={syncConfirm.open}
        objectName={syncConfirm.folder?.name ?? ''}
        currentData={syncConfirm.currentData}
        match={syncConfirm.match}
        isLoading={syncConfirm.isLoading}
        onApply={handleApplySyncMatch}
        onEditManually={handleCloseSyncConfirm}
        onClose={handleCloseSyncConfirm}
      />
    </>
  );
}
