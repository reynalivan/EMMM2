import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

export interface IniChange {
  label: string;
  filename: string;
  oldValue: string;
  newValue: string;
}

export interface MetadataChange {
  label: string;
  oldValue: string;
  newValue: string;
}

interface UnsavedIniChangesModalProps {
  open: boolean;
  isSaving: boolean;
  modName?: string;
  categoryName?: string;
  changedIniFields?: IniChange[];
  changedMetadataFields?: MetadataChange[];
  onSave: () => void;
  onDiscard: () => void;
  onCancel: () => void;
}

export default function UnsavedIniChangesModal({
  open,
  isSaving,
  modName,
  categoryName,
  changedIniFields,
  changedMetadataFields,
  onSave,
  onDiscard,
  onCancel,
}: UnsavedIniChangesModalProps) {
  const { t } = useTranslation(['preview', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const iniFields = changedIniFields ?? [];
  const metadataFields = changedMetadataFields ?? [];

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  const hasChanges = iniFields.length > 0 || metadataFields.length > 0;

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onCancel}>
      <div className="modal-box w-full max-w-lg border border-base-content/10 bg-base-100 p-5 shadow-2xl">
        <h3 className="text-base font-semibold text-base-content">{t('modals.unsaved.title')}</h3>
        <p className="mt-2 text-sm text-base-content/70">
          {t('modals.unsaved.description', {
            modName: modName || t('modals.unsaved.this_mod'),
            categorySuffix: categoryName ? ` (${categoryName})` : '',
          })}
        </p>

        {hasChanges && (
          <div className="mt-4 max-h-48 overflow-y-auto rounded-lg border border-base-content/10 bg-base-200/50 p-3 text-sm">
            {metadataFields.length > 0 && (
              <div className="mb-3 last:mb-0">
                <p className="mb-1 font-semibold text-base-content/80">
                  {t('modals.unsaved.metadata_edits')}
                </p>
                <ul className="space-y-1">
                  {metadataFields.map((c, i) => (
                    <li key={i} className="flex gap-2 text-base-content/70">
                      <span className="font-medium min-w-20">{c.label}:</span>
                      <span
                        className="truncate"
                        title={t('modals.unsaved.change_title', {
                          oldValue: c.oldValue || t('modals.unsaved.empty_value'),
                          newValue: c.newValue || t('modals.unsaved.empty_value'),
                        })}
                      >
                        <span className="line-through opacity-60 mr-1">
                          {c.oldValue || t('modals.unsaved.empty_value')}
                        </span>
                        <span className="text-success">
                          &rarr; {c.newValue || t('modals.unsaved.empty_value')}
                        </span>
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {iniFields.length > 0 && (
              <div className="mb-3 last:mb-0">
                <p className="mb-1 font-semibold text-base-content/80">
                  {t('modals.unsaved.ini_edits')}
                </p>
                <ul className="space-y-1">
                  {iniFields.map((c, i) => (
                    <li key={i} className="flex gap-2 text-base-content/70">
                      <span className="font-medium min-w-20" title={c.filename}>
                        {c.label}:
                      </span>
                      <span
                        className="truncate"
                        title={t('modals.unsaved.change_title', {
                          oldValue: c.oldValue || t('modals.unsaved.empty_value'),
                          newValue: c.newValue || t('modals.unsaved.empty_value'),
                        })}
                      >
                        <span className="line-through opacity-60 mr-1">
                          {c.oldValue || t('modals.unsaved.empty_value')}
                        </span>
                        <span className="text-success">
                          &rarr; {c.newValue || t('modals.unsaved.empty_value')}
                        </span>
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}

        <div className="modal-action mt-5">
          <button className="btn btn-sm btn-ghost" onClick={onCancel} disabled={isSaving}>
            {t('common:actions.cancel')}
          </button>
          <button className="btn btn-sm btn-warning" onClick={onDiscard} disabled={isSaving}>
            {t('common:actions.discard')}
          </button>
          <button className="btn btn-sm btn-primary" onClick={onSave} disabled={isSaving}>
            {isSaving ? t('modals.unsaved.saving') : t('common:actions.save')}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel} disabled={isSaving}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>
  );
}
