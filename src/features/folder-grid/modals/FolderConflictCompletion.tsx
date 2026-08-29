import { CheckCircle2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface Props {
  resolved: number;
  total: number;
  onClose: () => void;
}

export default function FolderConflictCompletion({ resolved, total, onClose }: Props) {
  const { t } = useTranslation('folder_grid');

  return (
    <div className="grid min-h-full place-items-center py-10 text-center">
      <div className="max-w-md">
        <span className="mx-auto grid size-14 place-items-center rounded-full bg-success/10 text-success">
          <CheckCircle2 size={30} aria-hidden />
        </span>
        <h3 className="mt-4 text-lg font-semibold">{t('conflict_manager.complete_title')}</h3>
        <p className="mt-1 text-sm text-base-content/60">
          {t('conflict_manager.complete_summary', { resolved, total })}
        </p>
        <p className="mt-1 text-sm text-base-content/60">
          {t('conflict_manager.complete_description')}
        </p>
        <button className="btn btn-success btn-sm mt-5" onClick={onClose}>
          {t('conflict_manager.close')}
        </button>
      </div>
    </div>
  );
}
