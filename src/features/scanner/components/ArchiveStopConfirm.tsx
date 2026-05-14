import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ArchiveStopConfirmProps {
  isOpen: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export default function ArchiveStopConfirm({
  isOpen,
  onCancel,
  onConfirm,
}: ArchiveStopConfirmProps) {
  const { t } = useTranslation(['scanner']);

  if (!isOpen) {
    return null;
  }

  return (
    <div className="absolute inset-0 bg-overlay-mask backdrop-blur-sm flex items-center justify-center z-50 rounded-lg">
      <div className="bg-base-200 border border-base-300 p-6 rounded-xl shadow-xl max-w-sm flex flex-col gap-4">
        <div className="flex items-center gap-3 text-error">
          <AlertTriangle size={24} />
          <h3 className="font-bold text-lg">{t('extract.stop_confirm_title')}</h3>
        </div>
        <p className="text-sm">{t('extract.stop_confirm_desc')}</p>
        <div className="flex justify-end gap-2 mt-2">
          <button className="btn btn-ghost btn-sm" onClick={onCancel}>
            {t('extract.action_cancel')}
          </button>
          <button className="btn btn-error btn-sm" onClick={onConfirm}>
            {t('extract.action_yes_stop')}
          </button>
        </div>
      </div>
    </div>
  );
}
