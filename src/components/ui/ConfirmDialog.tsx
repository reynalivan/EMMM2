/**
 * Reusable confirmation dialog using DaisyUI modal + HTML <dialog>.
 * Covers: NC-3.3-02 (Delete non-empty folder prompt)
 */

import { useRef, useEffect } from 'react';
import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  cancelLabel,
  danger = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useTranslation(['common']);
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

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onCancel}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-sm">
        {/* Icon + Title */}
        <div className="flex items-start gap-3">
          <div
            className={`p-2 rounded-lg ${
              danger ? 'bg-error/10 text-error' : 'bg-warning/10 text-warning'
            }`}
          >
            <AlertTriangle size={20} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base text-base-content">{title}</h3>
            <p className="text-sm text-base-content/60 mt-1 leading-relaxed">{message}</p>
          </div>
        </div>

        {/* Actions */}
        <div className="modal-action mt-4">
          <button className="btn btn-sm btn-ghost" onClick={onCancel}>
            {cancelLabel || t('actions.cancel')}
          </button>
          <button
            className={`btn btn-sm ${danger ? 'btn-error' : 'btn-warning'}`}
            onClick={onConfirm}
          >
            {confirmLabel || t('actions.confirm')}
          </button>
        </div>
      </div>

      {/* Backdrop closes on click */}
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel}>{t('actions.close')}</button>
      </form>
    </dialog>
  );
}
