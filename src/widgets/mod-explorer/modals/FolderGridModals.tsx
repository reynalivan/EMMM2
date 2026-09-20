import type { MoveStatus } from '@/entities/mod';
import type { ModFolder } from '@/entities/game-object';
import { MoveToObjectDialog } from '@/features/mod-runtime';
import ConfirmDialog from '../../../shared/ui/components/ui/ConfirmDialog';
import IgnoreManagementModal from './IgnoreManagementModal';
import { ActiveModContextDialog, BulkTagModal } from '@/features/mod-runtime';
import type { ObjectSummary } from '@/entities/game-object';
import { useTranslation } from 'react-i18next';
import CreateFolderDialog from './CreateFolderDialog';

export interface FolderGridModalsProps {
  moveDialog: { open: boolean; folder: ModFolder | null };
  closeMoveDialog: () => void;
  handleMoveToObject: (
    folder: ModFolder,
    targetId: string,
    status: MoveStatus,
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
  handleBulkTagSubmit: (tags: string[]) => void;
  selectionCount: number;
  bulkMovePaths: string[] | null;
  handleBulkMoveSubmit: (
    targetId: string,
    status: MoveStatus,
    targetSubpath: string | null,
  ) => Promise<void>;
  clearBulkMovePaths: () => void;
  isIgnoreManagementOpen: boolean;
  setIsIgnoreManagementOpen: (open: boolean) => void;
  activeContextDialog: { open: boolean; folder: ModFolder | null; isProcessing: boolean };
  handleActiveContextCancel: () => void;
  handleActiveContextSubmit: () => void;
  currentPath?: string;
  objects: ObjectSummary[];
  createFolderOpen: boolean;
  existingFolderNames: string[];
  isCreatingFolder: boolean;
  closeCreateFolderDialog: () => void;
  handleCreateFolder: (folderName: string) => Promise<void>;
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
  handleBulkTagSubmit,
  selectionCount,
  bulkMovePaths,
  handleBulkMoveSubmit,
  clearBulkMovePaths,
  isIgnoreManagementOpen,
  setIsIgnoreManagementOpen,
  activeContextDialog,
  handleActiveContextCancel,
  handleActiveContextSubmit,
  currentPath,
  objects,
  createFolderOpen,
  existingFolderNames,
  isCreatingFolder,
  closeCreateFolderDialog,
  handleCreateFolder,
}: FolderGridModalsProps) {
  const { t } = useTranslation(['grid']);

  return (
    <>
      {/* Move To Object Dialog */}
      {moveDialog.open && moveDialog.folder && (
        <MoveToObjectDialog
          isOpen={moveDialog.open}
          onClose={closeMoveDialog}
          targetModPaths={bulkMovePaths ?? [currentPath ?? moveDialog.folder.path]}
          showSuccessToast={!bulkMovePaths}
          currentObjectId={moveDialog.folder.owner_object_id ?? undefined}
          objects={objects}
          onSubmit={async (targetId: string, status: MoveStatus, targetSubpath: string | null) => {
            if (!moveDialog.folder) return;
            if (bulkMovePaths) {
              await handleBulkMoveSubmit(targetId, status, targetSubpath);
              clearBulkMovePaths();
              return;
            }
            const targetPaths = bulkMovePaths ?? [currentPath ?? moveDialog.folder.path];
            await handleMoveToObject(
              moveDialog.folder,
              targetId,
              status,
              targetSubpath,
              targetPaths,
            );
            clearBulkMovePaths();
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
        title={t('modals.bulk_delete_title', { count: selectionCount })}
        message={t('modals.bulk_delete_msg', { count: selectionCount })}
        confirmLabel={t('modals.bulk_delete_confirm_btn')}
        danger
        onConfirm={handleBulkDeleteConfirm}
        onCancel={() => setBulkDeleteConfirm(false)}
      />

      <BulkTagModal
        open={bulkTagOpen}
        mode="add"
        existingTags={[]}
        onSubmit={handleBulkTagSubmit}
        onClose={() => setBulkTagOpen(false)}
      />

      {/* Ignore Management Modal */}
      <IgnoreManagementModal
        open={isIgnoreManagementOpen}
        onClose={() => setIsIgnoreManagementOpen(false)}
      />

      {/* Shared active-context dialog: keep one viewport-level instance only. */}
      <ActiveModContextDialog
        key={activeContextDialog.folder?.path || 'dialog-hidden'}
        open={activeContextDialog.open}
        modName={activeContextDialog.folder?.name ?? ''}
        targetSafeStatus={!(activeContextDialog.folder?.is_safe ?? false)}
        isProcessing={activeContextDialog.isProcessing}
        onCancel={handleActiveContextCancel}
        onConfirm={handleActiveContextSubmit}
      />

      <CreateFolderDialog
        open={createFolderOpen}
        existingFolderNames={existingFolderNames}
        isCreating={isCreatingFolder}
        onClose={closeCreateFolderDialog}
        onSubmit={handleCreateFolder}
      />
    </>
  );
}
