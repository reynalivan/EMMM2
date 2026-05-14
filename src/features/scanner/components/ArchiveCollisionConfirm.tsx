import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ArchiveCollisionConfirmProps {
  isOpen: boolean;
  overwriteTargets: string[];
  onCancel: () => void;
  onConfirm: () => void;
}

export default function ArchiveCollisionConfirm({
  isOpen,
  overwriteTargets,
  onCancel,
  onConfirm,
}: ArchiveCollisionConfirmProps) {
  const { t } = useTranslation(['scanner']);

  if (!isOpen) {
    return null;
  }

  return (
    <div className="absolute inset-0 bg-overlay-mask backdrop-blur-sm flex items-center justify-center z-50 rounded-lg">
      <div className="bg-base-200 border border-base-300 p-6 rounded-xl shadow-xl max-w-sm flex flex-col gap-4">
        <div className="flex items-center gap-3 text-warning">
          <AlertTriangle size={24} />
          <h3 className="font-bold text-lg">{t('extract.overwrite_confirm_title')}</h3>
        </div>
        <p className="text-sm">
          {t('extract.overwrite_confirm_desc', { count: overwriteTargets.length })}
        </p>
        <ul className="text-sm list-disc list-inside max-h-32 overflow-y-auto bg-base-300/50 rounded-lg p-2">
          {overwriteTargets.map((name) => (
            <li key={name} className="font-mono text-warning">
              {name}
            </li>
          ))}
        </ul>
        <div className="flex justify-end gap-2 mt-2">
          <button className="btn btn-ghost btn-sm" onClick={onCancel}>
            {t('extract.action_cancel')}
          </button>
          <button className="btn btn-warning btn-sm" onClick={onConfirm}>
            {t('extract.action_yes_overwrite')}
          </button>
        </div>
      </div>
    </div>
  );
}
