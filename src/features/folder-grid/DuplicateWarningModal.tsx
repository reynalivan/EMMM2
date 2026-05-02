/**
 * Warning modal shown when enabling a mod that conflicts with
 * other enabled mods for the same character/object.
 * Covers: US-5.6 (Duplicate Character Warning)
 */

import { useRef, useEffect } from 'react';
import { AlertTriangle, Zap, ShieldAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { DuplicateInfo } from '../../types/scanner';

interface DuplicateWarningModalProps {
  open: boolean;
  targetName: string;
  duplicates: DuplicateInfo[];
  onForceEnable: () => void;
  onEnableOnlyThis: () => void;
  onCancel: () => void;
}

export default function DuplicateWarningModal({
  open,
  targetName,
  duplicates,
  onForceEnable,
  onEnableOnlyThis,
  onCancel,
}: DuplicateWarningModalProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
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
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-md">
        {/* Header */}
        <div className="flex items-start gap-3">
          <div className="p-2 rounded-lg bg-warning/10 text-warning">
            <AlertTriangle size={20} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base text-base-content">
              {t('duplicate_warning.title')}
            </h3>
            <p className="text-sm text-base-content/60 mt-1 leading-relaxed">
              {t('duplicate_warning.description', { targetName })}
            </p>
          </div>
        </div>

        {/* Duplicate list */}
        {duplicates.length > 0 && (
          <div className="mt-3 bg-base-200/50 rounded-lg p-3 border border-base-content/5">
            <p className="text-xs font-medium text-base-content/40 uppercase tracking-wider mb-2">
              {t('duplicate_warning.currently_enabled')}
            </p>
            <ul className="space-y-1">
              {duplicates.map((d) => (
                <li key={d.mod_id} className="text-sm text-base-content/70 flex items-center gap-2">
                  <ShieldAlert size={12} className="text-warning shrink-0" />
                  <span className="truncate">{d.actual_name}</span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Actions */}
        <div className="modal-action mt-4 flex-wrap gap-2">
          <button className="btn btn-sm btn-ghost" onClick={onCancel}>
            {t('common:actions.cancel')}
          </button>
          <button className="btn btn-sm btn-primary gap-1" onClick={onEnableOnlyThis}>
            <Zap size={14} />
            {t('duplicate_warning.enable_only_this')}
          </button>
          <button className="btn btn-sm btn-warning btn-outline gap-1" onClick={onForceEnable}>
            {t('duplicate_warning.force_enable')}
          </button>
        </div>
      </div>

      {/* Backdrop */}
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
