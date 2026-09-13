import { useEffect, useMemo, useRef, useState, type FormEvent, type MouseEvent } from 'react';
import { FolderPlus, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';

interface CreateFolderDialogProps {
  open: boolean;
  existingFolderNames: string[];
  isCreating: boolean;
  onClose: () => void;
  onSubmit: (folderName: string) => Promise<void>;
}

export default function CreateFolderDialog({
  open,
  existingFolderNames,
  isCreating,
  onClose,
  onSubmit,
}: CreateFolderDialogProps) {
  const { t } = useTranslation(['grid', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [folderName, setFolderName] = useState('');

  useDialogSync(dialogRef, open);

  useEffect(() => {
    if (open) {
      setFolderName('');
    }
  }, [open]);

  const validationMessage = useMemo(() => {
    const trimmedName = folderName.trim();
    if (!trimmedName) {
      return t('modals.create_folder_name_required');
    }
    if (
      existingFolderNames.some(
        (existingName) =>
          existingName.localeCompare(trimmedName, undefined, { sensitivity: 'accent' }) === 0,
      )
    ) {
      return t('modals.create_folder_name_duplicate');
    }
    return null;
  }, [existingFolderNames, folderName, t]);

  const handleClose = () => {
    if (!isCreating) {
      onClose();
    }
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (validationMessage) {
      return;
    }

    try {
      await onSubmit(folderName.trim());
      onClose();
    } catch {
      // The mutation keeps the dialog open and has already shown its actionable error toast.
    }
  };

  const handleBackdropClick = (event: MouseEvent<HTMLDialogElement>) => {
    if (event.target === event.currentTarget) {
      handleClose();
    }
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-middle"
      aria-labelledby="create-folder-title"
      onCancel={(event) => {
        event.preventDefault();
        handleClose();
      }}
      onClose={handleClose}
      onClick={handleBackdropClick}
    >
      <div className="modal-box w-11/12 max-w-sm border border-base-content/10 bg-base-100 shadow-2xl">
        <button
          type="button"
          className="btn btn-ghost btn-sm btn-square absolute right-3 top-3"
          onClick={handleClose}
          disabled={isCreating}
          aria-label={t('common:actions.close')}
        >
          <X size={16} />
        </button>

        <div className="flex items-center gap-3 pr-8">
          <div className="rounded-lg bg-primary/10 p-2 text-primary">
            <FolderPlus size={18} aria-hidden="true" />
          </div>
          <div>
            <h2 id="create-folder-title" className="text-base font-semibold text-base-content">
              {t('modals.create_folder_title')}
            </h2>
            <p className="mt-0.5 text-sm text-base-content/60">
              {t('modals.create_folder_description')}
            </p>
          </div>
        </div>

        <form className="mt-5" onSubmit={handleSubmit}>
          <label className="form-control w-full">
            <span className="label pb-1 text-sm font-medium text-base-content">
              {t('modals.create_folder_name_label')}
            </span>
            <input
              type="text"
              className={`input input-bordered w-full ${validationMessage ? 'input-error' : ''}`}
              value={folderName}
              onChange={(event) => setFolderName(event.target.value)}
              placeholder={t('modals.create_folder_name_placeholder')}
              autoFocus
              disabled={isCreating}
              aria-describedby={validationMessage ? 'create-folder-validation' : undefined}
            />
          </label>
          {validationMessage && (
            <p id="create-folder-validation" className="mt-2 text-xs text-error">
              {validationMessage}
            </p>
          )}

          <div className="modal-action mt-5 border-t border-base-content/10 pt-4">
            <button
              type="button"
              className="btn btn-sm btn-ghost"
              onClick={handleClose}
              disabled={isCreating}
            >
              {t('common:actions.cancel')}
            </button>
            <button
              type="submit"
              className="btn btn-sm btn-primary"
              disabled={Boolean(validationMessage) || isCreating}
            >
              {isCreating && (
                <span className="loading loading-spinner loading-xs" aria-hidden="true" />
              )}
              {t('modals.create_folder_submit')}
            </button>
          </div>
        </form>
      </div>
    </dialog>
  );
}
