import { AlertTriangle, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ConflictInfo } from '@/entities/workspace';

interface Props {
  conflicts: ConflictInfo[];
  onDismiss: () => void;
}

export default function ConflictToast({ conflicts, onDismiss }: Props) {
  const { t } = useTranslation(['scanner']);
  if (conflicts.length === 0) return null;

  return (
    <div className="workspace-transient-enter absolute right-0 top-full z-50 mt-2">
      <div className="alert alert-warning w-70 flex-row gap-2 px-3 py-2">
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
