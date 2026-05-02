import { useState } from 'react';
import { ShieldAlert, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ActiveModContextDialogProps {
  open: boolean;
  modName: string;
  targetSafeStatus: boolean;
  isProcessing: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ActiveModContextDialog({
  open,
  modName,
  targetSafeStatus,
  isProcessing,
  onConfirm,
  onCancel,
}: ActiveModContextDialogProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
  const [isChecked, setIsChecked] = useState(false);

  if (!open) return null;

  return (
    <dialog className={`modal ${open ? 'modal-open' : ''} bg-base-300/80 backdrop-blur-sm`}>
      <div className="modal-box border border-warning/20 shadow-2xl relative overflow-hidden">
        {isProcessing && (
          <div className="absolute inset-0 bg-base-100/50 backdrop-blur-sm z-50 flex flex-col items-center justify-center">
            <Loader2 size={32} className="animate-spin text-primary mb-4" />
            <p className="font-medium animate-pulse">{t('context.switching')}</p>
          </div>
        )}

        <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-4">
          <ShieldAlert size={20} />
          {t('context_dialog.title')}
        </h3>

        <div className="bg-base-200/50 rounded-lg p-4 mb-6 border border-base-content/5">
          <p className="text-sm mb-2">
            {t('context_dialog.move_message', {
              modName,
              target:
                targetSafeStatus
                  ? t('context_dialog.target_safe')
                  : t('context_dialog.target_unsafe'),
            })}
          </p>
          <p className="text-sm text-base-content/70">{t('context_dialog.active_warning')}</p>
        </div>

        <label className="label cursor-pointer justify-start gap-4 bg-base-200 p-4 rounded-lg border border-base-content/10 hover:border-primary/30 transition-colors">
          <input
            type="checkbox"
            className="checkbox checkbox-primary"
            checked={isChecked}
            onChange={(e) => setIsChecked(e.target.checked)}
            disabled={isProcessing}
          />
          <span className="label-text font-medium text-sm">
            {t('context_dialog.confirm_checkbox')}
          </span>
        </label>

        <div className="modal-action mt-6">
          <button className="btn btn-ghost" onClick={onCancel} disabled={isProcessing}>
            {t('common:actions.cancel')}
          </button>
          <button
            className="btn btn-warning"
            disabled={!isChecked || isProcessing}
            onClick={onConfirm}
          >
            {t('context_dialog.confirm_action')}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel} disabled={isProcessing}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>
  );
}
