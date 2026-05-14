import ArchiveModal from '../scanner/components/ArchiveModal';
import BulkTagModal from './BulkTagModal';
import DropConfirmModal, { type DropValidation } from './DropConfirmModal';
import type { WorkspaceObjectNode } from '../../types/workspace';
import type { ArchiveInfo } from '../../types/scanner';
import type { PendingDropContext } from './useObjHandlersArchive';

interface BulkTagModalState {
  open: boolean;
  mode: 'add' | 'remove';
}

interface ArchiveModalState {
  open: boolean;
  archives: ArchiveInfo[];
  isExtracting: boolean;
  error: string | null;
  passwordError: { path: string; message: string } | null;
  extractProgress: { current: number; total: number } | null;
  fileProgress: { fileName: string; fileIndex: number; totalFiles: number } | null;
  pendingDropContext: PendingDropContext | null;
}

interface ExtractOptions {
  autoRename?: boolean;
  disableByDefault?: boolean;
  folderNames?: Record<string, string>;
  unpackNested?: boolean;
}

interface ObjectListAuxiliaryModalsProps {
  dropValidation: DropValidation | null;
  onMoveAnyway: () => void;
  onMoveToSuggested: () => void;
  onCancelDrop: () => void;
  onSkipValidation: () => void;
  archiveModal: ArchiveModalState;
  objects: WorkspaceObjectNode[];
  onArchiveExtractSubmit: (
    selectedPaths: string[],
    passwords: Record<string, string>,
    options?: ExtractOptions,
  ) => Promise<void>;
  onArchiveExtractSkip: () => void;
  onStopExtraction: () => void;
  bulkTagModal: BulkTagModalState;
  selectedIds: Set<string>;
  onBulkAddTags: (ids: Set<string>, tags: string[]) => Promise<void>;
  onBulkRemoveTags: (ids: Set<string>, tags: string[]) => Promise<void>;
  onCloseBulkTagModal: () => void;
  onClearBulkSelection: () => void;
}

function parseObjectTags(tags: string | null | undefined): string[] {
  if (!tags) {
    return [];
  }

  try {
    const parsed = JSON.parse(tags);
    return Array.isArray(parsed)
      ? parsed.filter((tag): tag is string => typeof tag === 'string')
      : [];
  } catch {
    return [];
  }
}

export default function ObjectListAuxiliaryModals({
  dropValidation,
  onMoveAnyway,
  onMoveToSuggested,
  onCancelDrop,
  onSkipValidation,
  archiveModal,
  objects,
  onArchiveExtractSubmit,
  onArchiveExtractSkip,
  onStopExtraction,
  bulkTagModal,
  selectedIds,
  onBulkAddTags,
  onBulkRemoveTags,
  onCloseBulkTagModal,
  onClearBulkSelection,
}: ObjectListAuxiliaryModalsProps) {
  const targetObjectName = archiveModal.pendingDropContext?.targetObjectId
    ? objects.find((object) => object.id === archiveModal.pendingDropContext?.targetObjectId)?.name
    : undefined;

  const existingTags = [...selectedIds].flatMap((id) =>
    parseObjectTags(objects.find((object) => object.id === id)?.tags),
  );

  return (
    <>
      <DropConfirmModal
        validation={dropValidation}
        onMoveAnyway={onMoveAnyway}
        onMoveToSuggested={onMoveToSuggested}
        onCancel={onCancelDrop}
        onSkipValidation={onSkipValidation}
      />

      <ArchiveModal
        key={archiveModal.archives.length > 0 ? archiveModal.archives[0].path : 'empty'}
        isOpen={archiveModal.open}
        archives={archiveModal.archives}
        isExtracting={archiveModal.isExtracting}
        error={archiveModal.error}
        passwordError={archiveModal.passwordError}
        extractProgress={archiveModal.extractProgress}
        fileProgress={archiveModal.fileProgress}
        onExtract={onArchiveExtractSubmit}
        onSkip={onArchiveExtractSkip}
        onStop={onStopExtraction}
        targetObjectName={targetObjectName}
      />

      <BulkTagModal
        open={bulkTagModal.open}
        mode={bulkTagModal.mode}
        existingTags={existingTags}
        onSubmit={(tags) => {
          const operation =
            bulkTagModal.mode === 'add'
              ? onBulkAddTags(selectedIds, tags)
              : onBulkRemoveTags(selectedIds, tags);
          operation.then(onClearBulkSelection);
        }}
        onClose={onCloseBulkTagModal}
      />
    </>
  );
}
