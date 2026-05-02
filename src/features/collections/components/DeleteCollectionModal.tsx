import { AlertTriangle, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface DeleteCollectionModalProps {
  isOpen: boolean;
  collectionName: string;
  isDeleting: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteCollectionModal({
  isOpen,
  collectionName,
  isDeleting,
  onConfirm,
  onCancel,
}: DeleteCollectionModalProps) {
  const { t } = useTranslation(['collections']);

  if (!isOpen) return null;

  return (
    <dialog
      open
      className="modal modal-bottom sm:modal-middle bg-base-300/80 backdrop-blur-sm z-[100]"
    >
      <div className="modal-box border border-error/20 bg-base-100 shadow-xl relative top-0 max-w-sm">
        <h3 className="font-bold text-lg text-error flex items-center gap-2 mb-4">
          <AlertTriangle size={20} />
          {t('list.modal.delete_title', 'Delete Collection')}
        </h3>

        <p className="text-sm text-base-content/80">
          {t(
            'list.modal.delete_confirm',
            'Are you sure you want to delete "{{name}}"? This action cannot be undone.',
            { name: collectionName },
          )}
        </p>

        <div className="modal-action">
          <button className="btn btn-ghost btn-sm" onClick={onCancel} disabled={isDeleting}>
            {t('common:actions.cancel', 'Cancel')}
          </button>
          <button className="btn btn-error btn-sm" onClick={onConfirm} disabled={isDeleting}>
            {isDeleting ? <Loader2 size={14} className="animate-spin" /> : null}
            {t('common:actions.delete', 'Delete')}
          </button>
        </div>
      </div>
      <form
        method="dialog"
        className="modal-backdrop bg-transparent"
        onClick={!isDeleting ? onCancel : undefined}
      >
        <button type="button" disabled={isDeleting}>
          close
        </button>
      </form>
    </dialog>
  );
}
