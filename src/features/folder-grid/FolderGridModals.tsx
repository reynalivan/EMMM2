import type { ModFolder } from '../../types/mod';
import MoveToObjectDialog from './MoveToObjectDialog';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import ObjectConflictModal from './ObjectConflictModal';
import IgnoreManagementModal from './IgnoreManagementModal';
import { BulkTagModal } from './BulkTagModal';
import PinEntryModal from '../safe-mode/PinEntryModal';
import ActiveModContextDialog from './ActiveModContextDialog';
import SyncConfirmModal from '../object-list/SyncConfirmModal';
import type { MatchedDbEntry } from '../../lib/bindings';
import type { ObjectSummary } from '../../types/object';
import { useTranslation } from 'react-i18next';
import { closeWorkspaceDialog } from '../workspace-runtime/state/workspaceDialogs';
import { useWorkspaceRuntimeSelector } from '../workspace-runtime/state/workspaceStoreBridge';

export interface FolderGridModalsProps {
  moveDialog: { open: boolean; folder: ModFolder | null };
  closeMoveDialog: () => void;
  handleMoveToObject: (
    folder: ModFolder,
    targetId: string,
    status: 'disabled' | 'only-enable' | 'keep',
    targetSubpath?: string | null,
    targetModPaths?: string[],
  ) => Promise<void> | void;
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
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const duplicateConflict =
    dialogState.kind === 'duplicateConflict'
      ? { open: true, folder: dialogState.folder, duplicates: dialogState.duplicates }
      : { open: false, folder: null, duplicates: [] };

  return (
    <>
      {/* Move To Object Dialog */}
      {moveDialog.open && moveDialog.folder && (
        <MoveToObjectDialog
          isOpen={moveDialog.open}
          onClose={closeMoveDialog}
          targetModPaths={
            gridSelection.size > 1
              ? Array.from(gridSelection)
              : [currentPath ?? moveDialog.folder.path]
          }
          currentObjectId={objectId || undefined}
          objects={objects}
          onSubmit={async (
            targetId: string,
            status: 'disabled' | 'only-enable' | 'keep',
            targetSubpath: string | null,
          ) => {
            if (!moveDialog.folder) return;
            const targetPaths =
              gridSelection.size > 1
                ? Array.from(gridSelection)
                : [currentPath ?? moveDialog.folder.path];
            await handleMoveToObject(
              moveDialog.folder,
              targetId,
              status,
              targetSubpath,
              targetPaths,
            );
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
        open={duplicateConflict.open}
        folder={duplicateConflict.folder}
        duplicates={duplicateConflict.duplicates}
        onClose={() => closeWorkspaceDialog('duplicateConflict')}
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
