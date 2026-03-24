import { AlertTriangle, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ConflictInfo } from '../../../types/scanner';

interface Props {
  conflicts: ConflictInfo[];
  onDismiss: () => void;
}

export default function ConflictToast({ conflicts, onDismiss }: Props) {
  const { t } = useTranslation(['scanner']);
  if (conflicts.length === 0) return null;

  return (
    <div className="absolute top-full right-0 mt-2 z-50 animate-in slide-in-from-top-2 fade-in duration-300">
      <div className="alert alert-warning shadow-xl flex-row gap-2 py-2 px-3 w-70">
        <AlertTriangle className="w-5 h-5 shrink-0" />
        <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
          <span className="font-bold text-sm truncate" title={t('scanner:conflict_toast.title')}>
            {t('scanner:conflict_toast.title')}
          </span>
          <span className="text-[10px] truncate opacity-80">
            {t('scanner:conflict_toast.count', { count: conflicts.length })}
          </span>
        </div>
        <button
          className="btn btn-xs btn-ghost btn-circle shrink-0"
          onClick={(e) => {
            e.stopPropagation();
            onDismiss();
          }}
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}
