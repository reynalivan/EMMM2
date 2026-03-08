import { useEffect, useRef } from 'react';

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
  changedIniFields = [],
  changedMetadataFields = [],
  onSave,
  onDiscard,
  onCancel,
}: UnsavedIniChangesModalProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  const hasChanges = changedIniFields.length > 0 || changedMetadataFields.length > 0;

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onCancel}>
      <div className="modal-box w-full max-w-lg border border-base-content/10 bg-base-100 p-5 shadow-2xl">
        <h3 className="text-base font-semibold text-base-content">Unsaved Changes Detected</h3>
        <p className="mt-2 text-sm text-base-content/70">
          You have unsaved edits in <strong>{modName || 'this mod'}</strong>
          {categoryName ? ` (${categoryName})` : ''}. Save before navigating to keep these changes.
        </p>

        {hasChanges && (
          <div className="mt-4 max-h-48 overflow-y-auto rounded-lg border border-base-content/10 bg-base-200/50 p-3 text-sm">
            {changedMetadataFields.length > 0 && (
              <div className="mb-3 last:mb-0">
                <p className="mb-1 font-semibold text-base-content/80">Metadata Edits</p>
                <ul className="space-y-1">
                  {changedMetadataFields.map((c, i) => (
                    <li key={i} className="flex gap-2 text-base-content/70">
                      <span className="font-medium min-w-20">{c.label}:</span>
                      <span
                        className="truncate"
                        title={`${c.oldValue || '(empty)'} -> ${c.newValue || '(empty)'}`}
                      >
                        <span className="line-through opacity-60 mr-1">
                          {c.oldValue || '(empty)'}
                        </span>
                        <span className="text-success">&rarr; {c.newValue || '(empty)'}</span>
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {changedIniFields.length > 0 && (
              <div className="mb-3 last:mb-0">
                <p className="mb-1 font-semibold text-base-content/80">INI Configuration Edits</p>
                <ul className="space-y-1">
                  {changedIniFields.map((c, i) => (
                    <li key={i} className="flex gap-2 text-base-content/70">
                      <span className="font-medium min-w-20" title={c.filename}>
                        {c.label}:
                      </span>
                      <span
                        className="truncate"
                        title={`${c.oldValue || '(empty)'} -> ${c.newValue || '(empty)'}`}
                      >
                        <span className="line-through opacity-60 mr-1">
                          {c.oldValue || '(empty)'}
                        </span>
                        <span className="text-success">&rarr; {c.newValue || '(empty)'}</span>
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
            Cancel
          </button>
          <button className="btn btn-sm btn-warning" onClick={onDiscard} disabled={isSaving}>
            Discard
          </button>
          <button className="btn btn-sm btn-primary" onClick={onSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel} disabled={isSaving}>
          close
        </button>
      </form>
    </dialog>
  );
}
