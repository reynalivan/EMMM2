import { parseTagList } from '../utils/bulkSummary';
import { BulkTagModal } from '@/features/mod-runtime';
import type { WorkspaceObjectNode } from '@/entities/workspace';

interface Props {
  objects: WorkspaceObjectNode[];
  bulkTagModal: { open: boolean; mode: 'add' | 'remove' };
  selectedIds: Set<string>;
  onBulkAddTags: (ids: Set<string>, tags: string[]) => Promise<void>;
  onBulkRemoveTags: (ids: Set<string>, tags: string[]) => Promise<void>;
  onCloseBulkTagModal: () => void;
  onClearBulkSelection: () => void;
}

export default function ObjectListAuxiliaryModals({
  objects,
  bulkTagModal,
  selectedIds,
  onBulkAddTags,
  onBulkRemoveTags,
  onCloseBulkTagModal,
  onClearBulkSelection,
}: Props) {
  const existingTags = [...selectedIds].flatMap((id) =>
    parseTagList(objects.find((object) => object.id === id)?.tags),
  );

  return (
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
  );
}
